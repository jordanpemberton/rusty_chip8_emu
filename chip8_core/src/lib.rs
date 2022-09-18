pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const FONTSET_SIZE: usize = 80;
const START_ADDR: u16 = 0x200;  // first 512 addresses are left empty, can be used to store sprite data for font charaters.

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0,   // 0
    0x20, 0x60, 0x20, 0x20, 0x70,   // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0,   // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0,   // 3
    0x90, 0x90, 0xF0, 0x10, 0x10,   // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0,   // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0,   // 6
    0xF0, 0x10, 0x20, 0x40, 0x40,   // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0,   // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0,   // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90,   // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0,   // B
    0xF0, 0x80, 0x80, 0x80, 0xF0,   // C
    0xE0, 0x90, 0x90, 0x90, 0xE0,   // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0,   // E
    0xF0, 0x80, 0xF0, 0x80, 0x80    // F
];

pub struct Emulator {
    program_counter: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    vreg: [u8; NUM_REGS],
    i_register: u16,
    stack_pointer: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    delay_timer: u8,
    sound_timer: u8
}

impl Emulator {
    pub fn new() -> Self {
        let mut emulator = Self {
            program_counter: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            vreg: [0; NUM_REGS],
            i_register: 0,
            stack_pointer: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            delay_timer: 0,
            sound_timer: 0
        };
        emulator.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        emulator
    }

    pub fn reset(&mut self) {
        self.program_counter = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.vreg = [0; NUM_REGS];
        self.i_register = 0;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    fn push(&mut self, value: u16) {
        self.stack[self.stack_pointer as usize] = value;
        self.stack_pointer += 1;
    }

    fn pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer as usize]
    }

    // fetch 16-bit opcode stored at current Program Counter.
    // Values are stored in RAM as 8-bit values, so we fetch two,
    // and combine them as Big Endian, then increment PC by 2 bytes.
    pub fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.program_counter as usize] as u16;
        let lower_byte = self.ram[(self.program_counter + 1) as usize] as u16;
        let operation = (higher_byte << 8) | lower_byte;
        self.program_counter += 2;
        operation
    }

    fn clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn return_from_subroutine(&mut self) {
        let return_address = self.pop();
        self.program_counter = return_address;
    }

    // 1
    fn jump_to(&mut self, address: u16) {
        self.program_counter = address;
    }

    // 2
    fn call(&mut self, address: u16) {
        self.push(self.program_counter);
        self.program_counter = address;
    }

    // 3
    // If v register == NN value, skip the next opcode (ie increment pc by 2).
    fn skip_if_same(&mut self, which_vreg: usize, value: u8) {
        if self.vreg[which_vreg] == value {
            self.program_counter += 2;
        }
    }

    // 4
    // If v register != NN value, skip the next opcode (ie increment pc by 2).
    fn skip_if_diff(&mut self, which_vreg: usize, value: u8) {
        if self.vreg[which_vreg] != value {
            self.program_counter += 2;
        }
    }

    // 5
    // If v register a == v register b, skip the next opcode (ie increment pc by 2).
    fn skip_if_same_compare(&mut self, which_vreg_a: usize, which_vreg_b: usize) {
        if self.vreg[which_vreg_a] == self.vreg[which_vreg_b] {
            self.program_counter += 2;
        }
    }

    // 6
    fn set(&mut self, which_vreg: usize, value: u8) {
        self.vreg[which_vreg] = value;
    }

    // 7
    // Need to account for the case of overflow, so can't use addition operation.
    // Note that the Chip8 carry flag is not modified by this operation.
    fn wrapping_add(&mut self, which_vreg: usize, value: u8) {
        self.vreg[which_vreg] = self.vreg[which_vreg].wrapping_add(value); // ?
    }

    // 8
    fn set_from_vreg(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] = self.vreg[which_vreg_source];
    }

    fn bitwise_or(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] |= self.vreg[which_vreg_source];
    }

    fn bitwise_and(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] &= self.vreg[which_vreg_source];
    }

    // fn bitwise_??(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
    //     self.vreg[which_vreg_target] ??= self.vreg[which_vreg_source];
    // }

    // Utilizes the VF flag register, which stores the carry flag.
    // Must handle overflow case.
    fn overflowing_add(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        let (value, carry) = self.vreg[which_vreg_target].overflowing_add(self.vreg[which_vreg_source]);
        let flag = if carry { 1 } else { 0 };

        self.vreg[which_vreg_target] = value;
        self.vreg[0xF] = flag; // vreg[16]?, the final 16th (0xf) register holds the flag register
    }

    // Also utilizes the VF flag register, which stores the carry flag.
    // Must handle overflow case.
    fn overflowing_subtract(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        let (value, borrow) = self.vreg[which_vreg_target].overflowing_sub(self.vreg[which_vreg_source]);
        let flag = if borrow { 0 } else { 1 };

        self.vreg[which_vreg_target] = value;
        self.vreg[0xF] = flag; // vreg[16]?, the final 16th (0xf) register holds the flag register
    }

    // Rightshift the v register by 1, and store the dropped bit in the VF register.
    fn bitwise_rightshift(&mut self, which_vreg: usize) {
        let least_signif_bit = self.vreg[which_vreg] & 1;
        self.vreg[which_vreg] >>= 1;
        self.vreg[0xF] = least_signif_bit;
    }

    // Execute the opcode.
    fn execute(&mut self, opcode: u16) {
        let digit1 = (opcode & 0xF000) >> 12;
        let digit2 = (opcode & 0x0F00) >> 8;
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = (opcode & 0x000F);

        match (digit2, digit2, digit3, digit4) {
            (0, 0, 0, 0) => return,
            (0, 0, 0xE, 0) => Emulator::clear_screen(self),
            (0, 0, 0xE, 0xE) => Emulator::return_from_subroutine(self),

            (1, _, _, _) => Emulator::jump_to(self, opcode & 0xFFF),
            (2, _, _, _) => Emulator::call(self, opcode & 0xFFF),

            (3, _, _, _) => Emulator::skip_if_same(self, digit2 as usize, (opcode & 0xFF) as u8),
            (4, _, _, _) => Emulator::skip_if_diff(self, digit2 as usize, (opcode & 0xFF) as u8),
            (5, _, _, _) => Emulator::skip_if_same_compare(self, digit2 as usize, digit3 as usize),

            (6, _, _, _) => Emulator::set(self, digit2 as usize, (opcode & 0xFF) as u8),
            (7, _, _, _) => Emulator::wrapping_add(self, digit2 as usize, (opcode & 0xFF) as u8),

            (8, _, _, 0) => Emulator::set_from_vreg(self, digit2 as usize, digit3 as usize),
            (8, _, _, 1) => Emulator::bitwise_or(self, digit2 as usize, digit3 as usize),
            (8, _, _, 2) => Emulator::bitwise_and(self, digit2 as usize, digit3 as usize),
            // (8, _, _, 3) => Emulator::bitwise_??(self, digit2 as usize, digit3 as usize), // TODO
            (8, _, _, 4) => Emulator::overflowing_add(self, digit2 as usize, digit3 as usize),
            (8, _, _, 5) => Emulator::overflowing_subtract(self, digit2 as usize, digit3 as usize),
            // (8, _, _, 6) => Emulator::bitwise_rightshift(self, digit2 as usize, digit3 as usize),

            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", opcode)
        }
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // BEEP
                // TODO audio
            }
            self.sound_timer -= 1;
        }
    }    

    pub fn tick(&mut self) {
        // Fetch
        let opcode = self.fetch();

        // Decode
        // Execute
    }
}



pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
