use rand::Rng;

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
    ireg: u16,
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
            ireg: 0,
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
        self.ireg = 0;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, index: usize, is_pressed: bool) {
        self.keys[index] = is_pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
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

        // Decode // Execute
        self.execute(opcode);
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
    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.program_counter as usize] as u16;
        let lower_byte = self.ram[(self.program_counter + 1) as usize] as u16;
        let operation = (higher_byte << 8) | lower_byte;
        self.program_counter += 2;
        operation
    }

    // 00E0 Clear screen
    fn clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    // 00EE
    fn return_from_subroutine(&mut self) {
        let return_address = self.pop();
        self.program_counter = return_address;
    }

    // 1NNN
    fn jump_to(&mut self, address: u16) {
        self.program_counter = address;
    }

    // 2NNN
    fn call(&mut self, address: u16) {
        self.push(self.program_counter);
        self.program_counter = address;
    }

    // 3XNN
    // If v register == NN value, skip the next opcode (ie increment pc by 2).
    fn skip_if_same(&mut self, which_vreg: usize, value: u8) {
        if self.vreg[which_vreg] == value {
            self.program_counter += 2;
        }
    }

    // 4XNN
    // If v register != NN value, skip the next opcode (ie increment pc by 2).
    fn skip_if_diff(&mut self, which_vreg: usize, value: u8) {
        if self.vreg[which_vreg] != value {
            self.program_counter += 2;
        }
    }

    // 5XY0
    // If v register a == v register b, skip the next opcode (ie increment pc by 2).
    fn skip_if_same_compare(&mut self, which_vreg_a: usize, which_vreg_b: usize) {
        if self.vreg[which_vreg_a] == self.vreg[which_vreg_b] {
            self.program_counter += 2;
        }
    }

    // 6XNN
    fn set_vreg(&mut self, which_vreg: usize, value: u8) {
        self.vreg[which_vreg] = value;
    }

    // 7XNN
    // Need to account for the case of overflow, so can't use addition operation.
    // Note that the Chip8 carry flag is not modified by this operation.
    fn wrapping_add(&mut self, which_vreg: usize, value: u8) {
        self.vreg[which_vreg] = self.vreg[which_vreg].wrapping_add(value);
    }

    // 8XY0
    fn set_from_vreg(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] = self.vreg[which_vreg_source];
    }

    // 8XY1
    fn bitwise_or(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] |= self.vreg[which_vreg_source];
    }

    // 8XY2
    fn bitwise_and(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] &= self.vreg[which_vreg_source];
    }

    // 8XY3
    fn bitwise_xor(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        self.vreg[which_vreg_target] ^= self.vreg[which_vreg_source];
    }

    // 8XY4
    // Utilizes the VF flag register, which stores the carry flag.
    // Must handle overflow case.
    fn overflowing_add(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        let (value, carry) = self.vreg[which_vreg_target].overflowing_add(self.vreg[which_vreg_source]);
        let flag = if carry { 1 } else { 0 };

        self.vreg[which_vreg_target] = value;
        self.vreg[0xF] = flag; // same as vreg[16]?, the final 16th (0xf) register holds the flag register
    }

    // 8XY5
    // Also utilizes the VF flag register, which stores the carry flag.
    // Must handle overflow case.
    fn overflowing_subtract(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        let (value, borrow) = self.vreg[which_vreg_target].overflowing_sub(self.vreg[which_vreg_source]);
        let flag = if borrow { 0 } else { 1 };

        self.vreg[which_vreg_target] = value;
        self.vreg[0xF] = flag; // vreg[16]?, the final 16th (0xf) register holds the flag register
    }

    // 8XY6
    // Rightshift the v register by 1, and store the dropped bit in the VF register.
    fn rightshift(&mut self, which_vreg: usize) {
        let least_signif_bit = self.vreg[which_vreg] & 1;
        self.vreg[which_vreg] >>= 1;
        self.vreg[0xF] = least_signif_bit;
    }

    // 8XY7
    // Overflowing subtract but flipped, Clear VF if borrow
    fn overflowing_subtract_flipped(&mut self, which_vreg_target: usize, which_vreg_source: usize) {
        let (value, borrow) = self.vreg[which_vreg_source].overflowing_sub(self.vreg[which_vreg_target]);
        let flag = if borrow { 0 } else { 1 };

        self.vreg[which_vreg_target] = value;
        self.vreg[0xF] = flag; // vreg[16]?, the final 16th (0xf) register holds the flag register
    }

    // 8XYE
    // Left shift and store dropped bit in VF
    fn leftshift(&mut self, which_vreg: usize) {
        let most_signif_bit = (self.vreg[which_vreg] >> 7) & 1;
        self.vreg[which_vreg] <<= 1;
        self.vreg[0xF] = most_signif_bit;
    }

    // 9XY0
    // Skip if VX != VY
    // (Same as 5XY0 but with inequality.)
    fn skip_if_diff_compare(&mut self, which_vreg_a: usize, which_vreg_b: usize) {
        if self.vreg[which_vreg_a] != self.vreg[which_vreg_b] {
            self.program_counter += 2;
        }
    }

    // ANNN
    // I = 0xNNN
    fn set_ireg(&mut self, value: u16) {
        self.ireg = value;
    }

    // BNNN
    // Jump to V0 + 0xNNN
    fn jump_vreg0_to(&mut self, address: u16) {
        self.program_counter = (self.vreg[0] as u16) + address;
    }

    // CXNN
    // VX = rand() & 0xNN
    fn set_to_rand(&mut self, which_vreg: usize, value: u8) {
        let rng: u8 = rand::thread_rng().gen(); // random();
        self.vreg[which_vreg] = rng & value;
    }

    // DXYN
    // Draw sprite at (VX, VY)
    fn draw_sprite(&mut self, which_vreg_x: usize, which_vreg_y: usize, num_rows: u16) {
        let x_coord = self.vreg[which_vreg_x] as u16;
        let y_coord = self.vreg[which_vreg_y] as u16;
        let mut flipped = false;

        for y_line in 0..num_rows {
            let address = self.ireg + y_line as u16;
            let pixels = self.ram[address as usize];

            for x_line in 0..8 {
                if (pixels & (0b1000_0000 >> x_line)) != 0 {
                    let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                    let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                    let index = x + SCREEN_WIDTH * y;

                    flipped |= self.screen[index];
                    self.screen[index] ^= true;
                }
            }
        }

        // Set VF register
        if flipped {
            self.vreg[0xF] = 1;
        } else {
            self.vreg[0xF] = 0;
        }
    }

    // EX9E
    // Skip if key index in VX is pressed
    fn skip_if_key(&mut self, which_vreg: usize) {
        let vx = self.vreg[which_vreg];
        let key = self.keys[vx as usize];
        if key {
            self.program_counter += 2;
        }
    }

    // EXA1
    // Skip if not key
    fn skip_if_not_key(&mut self, which_vreg: usize) {
        let vx = self.vreg[which_vreg];
        let key = self.keys[vx as usize];
        if !key {
            self.program_counter += 2;
        }
    }

    // FX07
    // Set VX = Delay Timer
    fn set_vreg_to_delay_timer(&mut self, which_vreg: usize) {
        self.vreg[which_vreg] = self.delay_timer;
    }

    // FX0A
    // Wait for key, store index in VX
    fn wait_for_key(&mut self, which_vreg: usize) {
        let mut pressed = false;

        for i in 0..self.keys.len() {
            if self.keys[i] {
                self.vreg[which_vreg] = i as u8;
                pressed = true;
                break;
            }
        }

        // Block/wait if key not pressed, by repeating the opcode
        // (Not in an infinite loop because we must not block input.)
        if !pressed {
            self.program_counter -= 2;
        }
    }

    // FX15
    // Delay Timer = VX
    fn set_delay_timer(&mut self, which_vreg: usize) {
        self.delay_timer = self.vreg[which_vreg];
    }

    // FX18
    // Sound Timer = VX
    fn set_sound_timer(&mut self, which_vreg: usize) {
        self.sound_timer = self.vreg[which_vreg];
    }

    // FX1E
    // I += VX
    fn add_to_ireg(&mut self, which_vreg: usize) {
        let vx = self.vreg[which_vreg] as u16;
        self.ireg = self.ireg.wrapping_add(vx);
    }

    // FX29
    // Set i to address of font character in VX
    fn set_ireg_to_font_address(&mut self, which_vreg: usize) {
            let c = self.vreg[which_vreg] as u16;
            self.ireg = c * 5; // multiplied by 5 because each font occupies 5 bytes
    }

    fn easy_to_read_bcd(x: f32) -> [u8; 3] {
        let hundreds = (x / 100.0).floor() as u8;
        let tens = ((x / 10.0) % 10.0).floor() as u8;
        let ones = (x % 10.0) as u8;
        [hundreds, tens, ones]
    }

    // FX33
    // Store BCD encoding of VX into I, inclusive.
    /// BCD = Binary-Coded Decimal, to convert hex back into decimal.
    fn load_bcd_vreg_into_ireg(&mut self, which_vreg: usize) {
        let vx = self.vreg[which_vreg] as f32;

        let [hundreds, tens, ones] = Emulator::easy_to_read_bcd(vx);

        self.ram[self.ireg as usize] = hundreds;
        self.ram[(self.ireg + 1) as usize] = tens;
        self.ram[(self.ireg + 2) as usize] = ones;
    }

    // FX55
    // Load V0 through VX into I
    fn load_vreg_into_ireg(&mut self, n: usize) {
        let start_address = self.ireg as usize;
        for i in 0..=n {
            self.ram[start_address + i] = self.vreg[i];
        }
    }

    // FX65
    // Load I into V0 thru VX with RAM values starting at address 1, inclusive.
    fn load_ireg_into_vreg(&mut self, n: usize) {
        let start_address = self.ireg as usize;
        for i in 0..=n {
            self.vreg[i] = self.ram[start_address + i]
        }
    }

    // Execute the opcode.
    fn execute(&mut self, opcode: u16) {
        let digit1 = (opcode & 0xF000) >> 12;
        let digit2 = (opcode & 0x0F00) >> 8;
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            (0, 0, 0, 0) => return,
            (0, 0, 0xE, 0) => Emulator::clear_screen(self),
            (0, 0, 0xE, 0xE) => Emulator::return_from_subroutine(self),

            (1, _, _, _) => Emulator::jump_to(self, opcode & 0xFFF),
            (2, _, _, _) => Emulator::call(self, opcode & 0xFFF),
            (3, _, _, _) => Emulator::skip_if_same(self, digit2 as usize, (opcode & 0xFF) as u8),
            (4, _, _, _) => Emulator::skip_if_diff(self, digit2 as usize, (opcode & 0xFF) as u8),
            (5, _, _, _) => Emulator::skip_if_same_compare(self, digit2 as usize, digit3 as usize),
            (6, _, _, _) => Emulator::set_vreg(self, digit2 as usize, (opcode & 0xFF) as u8),
            (7, _, _, _) => Emulator::wrapping_add(self, digit2 as usize, (opcode & 0xFF) as u8),

            (8, _, _, 0) => Emulator::set_from_vreg(self, digit2 as usize, digit3 as usize),
            (8, _, _, 1) => Emulator::bitwise_or(self, digit2 as usize, digit3 as usize),
            (8, _, _, 2) => Emulator::bitwise_and(self, digit2 as usize, digit3 as usize),
            (8, _, _, 3) => Emulator::bitwise_xor(self, digit2 as usize, digit3 as usize),
            (8, _, _, 4) => Emulator::overflowing_add(self, digit2 as usize, digit3 as usize),
            (8, _, _, 5) => Emulator::overflowing_subtract(self, digit2 as usize, digit3 as usize),
            (8, _, _, 6) => Emulator::rightshift(self, digit2 as usize),
            (8, _, _, 7) => Emulator::overflowing_subtract_flipped(self, digit3 as usize, digit2 as usize),
            (8, _, _, 0xE) => Emulator::leftshift(self, digit3 as usize),

            (9, _, _, 0) => Emulator::skip_if_diff_compare(self, digit2 as usize, digit3 as usize),

            (0xA, _, _, _) => Emulator::set_ireg(self, opcode & 0xFFF),

            (0xB, _, _, _) => Emulator::jump_vreg0_to(self, opcode & 0xFFF),

            (0xC, _, _, _) => Emulator::set_to_rand(self, digit2 as usize, (opcode & 0xFF) as u8),

            (0xD, _, _, _) => Emulator::draw_sprite(self, digit2 as usize, digit3 as usize, digit4 as u16),

            (0xE, _, 9, 0xE) => Emulator::skip_if_key(self, digit2 as usize),
            (0xE, _, 0xA, 1) => Emulator::skip_if_not_key(self, digit2 as usize),

            (0xF, _, 0, 7) => Emulator::set_vreg_to_delay_timer(self, digit2 as usize),
            (0xF, _, 0, 0xA) => Emulator::wait_for_key(self, digit2 as usize),
            (0xF, _, 1, 5) => Emulator::set_delay_timer(self, digit2 as usize),
            (0xF, _, 1, 8) => Emulator::set_sound_timer(self, digit2 as usize),
            (0xF, _, 1, 0xE) => Emulator::add_to_ireg(self, digit2 as usize),
            (0xF, _, 2, 9) => Emulator::set_ireg_to_font_address(self, digit2 as usize),
            (0xF, _, 3, 3) => Emulator::load_bcd_vreg_into_ireg(self, digit2 as usize),
            (0xF, _, 5, 5) => Emulator::load_vreg_into_ireg(self, digit2 as usize),
            (0xF, _, 6, 5) => Emulator::load_ireg_into_vreg(self, digit2 as usize),

            (_, _, _, _) => unimplemented!("Unimplemented opcode: {:x} ({:x} {:x} {:x} {:x})", opcode, digit1, digit2, digit3, digit4)
        }
    }
}
