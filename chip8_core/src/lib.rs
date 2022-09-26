#![allow(non_snake_case)]

use rand::Rng;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const FONTSET_SIZE: usize = 80;
const START_ADDR: usize = 0x200;  // First 512 addresses are left empty, can be used to store sprite data for font characters.

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
    screen_changed: bool,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: usize,
    stack_pointer: usize,
    ireg: usize,
    vreg: [u8; NUM_REGS],
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT]
}

impl Emulator {
    pub fn new() -> Self {
        let mut emulator = Self {
            screen_changed: false,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: START_ADDR,
            stack_pointer: 0,
            ireg: 0,
            vreg: [0; NUM_REGS],
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT]
        };
        emulator.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        emulator
    }

    pub fn reset(&mut self) {
        self.screen_changed = false;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.program_counter = START_ADDR;
        self.stack_pointer = 0;
        self.ireg = 0;
        self.vreg = [0; NUM_REGS];
        self.stack = [0; STACK_SIZE];
        self.ram = [0; RAM_SIZE];
        self.keys = [false; NUM_KEYS];
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn load_data(&mut self, data: &[u8]) {
        let start = START_ADDR;
        let end = (START_ADDR) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn push(&mut self, value: u16) {
        self.stack[self.stack_pointer] = value;
        self.stack_pointer += 1;
    }

    fn pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer]
    }

    pub fn keypress(&mut self, ki: usize, is_pressed: bool) {
        self.keys[ki] = is_pressed;
    }

    // FETCH 16-bit opcode stored at current Program Counter.
    // Values are stored in RAM as 8-bit values, so we fetch two,
    // and combine them as Big Endian, then increment PC by 2 bytes.
    fn fetch_opcode(&mut self) -> u16 {
        let higher_byte = self.ram[self.program_counter] as u16;
        let lower_byte = self.ram[(self.program_counter + 1)] as u16;
        let opcode = (higher_byte << 8) | lower_byte;

        self.program_counter += 2;

        opcode
    }

    // TICK
    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // BEEP // TODO audio
            }
            self.sound_timer -= 1;
        }
    }

    pub fn tick(&mut self) {
        self.screen_changed = false;
        let opcode = self.fetch_opcode();
        self.execute(opcode);
    }

    // EXECUTE the opcode.
    fn execute(&mut self, opcode: u16) {
        let nibbles = (
            (opcode & 0xF000) >> 12 as u8,
            (opcode & 0x0F00) >> 8 as u8,
            (opcode & 0x00F0) >> 4 as u8,
            (opcode & 0x000F) as u8
        );

        let nnn = (opcode & 0x0FFF) as usize;
        let kk = (opcode & 0x00FF) as u8;
        let x = nibbles.1 as usize;
        let y = nibbles.2 as usize;
        let n = nibbles.3 as usize;

        match nibbles {
            (0, 0, 0, 0) => return,
            (0, 0, 0xE, 0) => self.op_00E0_cls(),
            (0, 0, 0xE, 0xE) => self.op_00EE_ret(),
            (1, _, _, _) => self.op_1NNN_jmp(nnn),
            (2, _, _, _) => self.op_2NNN_call(nnn),
            (3, _, _, _) => self.op_3XKK_se_vx_kk(x, kk),
            (4, _, _, _) => self.op_4XKK_sne_vx_kk(x, kk),
            (5, _, _, 0) => self.op_5XY0_se_vx_vy(x, y),
            (6, _, _, _) => self.op_6XKK_ld_vx_kk(x, kk),
            (7, _, _, _) => self.op_7XKK_add_vx_kk(x, kk),
            (8, _, _, 0) => self.op_8XY0_ld_vx_vy(x, y),
            (8, _, _, 1) => self.op_8XY1_or_vx_vy(x, y),
            (8, _, _, 2) => self.op_8XY2_and_vx_vy(x, y),
            (8, _, _, 3) => self.op_8XY3_xor_vx_vy(x, y),
            (8, _, _, 4) => self.op_8XY4_add_vx_vy(x, y),
            (8, _, _, 5) => self.op_8XY5_sub_vx_vy(x, y),
            (8, _, _, 6) => self.op_8XY6_shr_vx(x),
            (8, _, _, 7) => self.op_8XY7_subn_vx_vy(y, x),
            (8, _, _, 0xE) => self.op_8XYE_shl_vx(x),
            (9, _, _, 0) => self.op_9XY0_sne_vx_vy(x, y),
            (0xA, _, _, _) => self.op_ANNN_ld_i_nnn(nnn),
            (0xB, _, _, _) => self.op_BNNN_jmp_v0_nnn(nnn),
            (0xC, _, _, _) => self.op_CXKK_ld_vx_rand_and_kk(y, kk),
            (0xD, _, _, _) => self.op_DXYN_drw(x, y, n),
            (0xE, _, 9, 0xE) => self.op_EX9E_skp_vx(x),
            (0xE, _, 0xA, 1) => self.op_EXA1_sknp_vx(x),
            (0xF, _, 0, 7) => self.op_FX07_ld_vx_dt(x),
            (0xF, _, 0, 0xA) => self.op_FX0A_ld_vx_key(x),
            (0xF, _, 1, 5) => self.op_FX15_ld_dt_vx(x),
            (0xF, _, 1, 8) => self.op_FX18_ld_st_vx(x),
            (0xF, _, 1, 0xE) => self.op_FX1E_add_i_vx(x),
            (0xF, _, 2, 9) => self.op_FX29_ld_d_vx(x),
            (0xF, _, 3, 3) => self.op_FX33_ld_b_vx(x),
            (0xF, _, 5, 5) => self.op_FX55_ld_i_vx(x),
            (0xF, _, 6, 5) => self.op_FX65_ld_vx_i(x),
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {:?})", nibbles)
        }
    }

    // ======================== OPCODE INSTRUCTIONS ========================
    // nnn or addr - A 12-bit value, the lowest 12 bits of the instruction
    // n or nibble - A 4-bit value, the lowest 4 bits of the instruction
    // x - A 4-bit value, the lower 4 bits of the high byte of the instruction
    // y - A 4-bit value, the upper 4 bits of the low byte of the instruction
    // kk or byte - An 8-bit value, the lowest 8 bits of the instruction

    /// 000 - NOP
    /// No op, do nothing

    // 0NNN - SYS addr
    // Jump to machine code routine at NNN. Ignored by modern interpreters.

    // 00E0 - CLS
    // Clear screen
    fn op_00E0_cls(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.screen_changed = true;
    }

    // 00EE - RET
    // Return from subroutine (set program counter to the address at the top of the stack).
    fn op_00EE_ret(&mut self) {
        self.program_counter = self.pop() as usize;
    }

    // 1NNN - JP
    // Jump to address NNN.
    fn op_1NNN_jmp(&mut self, nnn: usize) {
        self.program_counter = nnn;
    }

    // 2NNN - CALL
    // Call subroutine at address NNN. Push current PC onto top of stack and set PC to NNN.
    fn op_2NNN_call(&mut self, nnn: usize) {
        self.push(self.program_counter as u16);
        self.program_counter = nnn;
    }

    fn skip_if(&mut self, condition: bool) {
        if condition {
            self.program_counter += 2;
        }
    }

    // 3XKK - SE Vx, KK
    // Skip next instruction if Vx == KK.
    fn op_3XKK_se_vx_kk(&mut self, vi: usize, kk: u8) {
        self.skip_if(self.vreg[vi] == kk);
    }

    // 4XKK - SNE Vx, KK
    // Skip next instruction if Vx != KK.
    fn op_4XKK_sne_vx_kk(&mut self, vi: usize, kk: u8) {
        self.skip_if(self.vreg[vi] != kk);
    }

    // 5XY0 - SE Vx, Vy
    // Skip next instruction if Vx == Vy.
    fn op_5XY0_se_vx_vy(&mut self, vi: usize, vj: usize) {
        self.skip_if(self.vreg[vi] == self.vreg[vj]);
    }

    // 6XKK - LD Vx, KK
    // Set Vx = KK.
    fn op_6XKK_ld_vx_kk(&mut self, vi: usize, kk: u8) {
        self.vreg[vi] = kk;
    }

    // 7XKK - ADD Vx, KK
    // Need to account for the case of overflow, so can't use addition operation.
    // Note that the Chip8 carry flag is not modified by this operation.
    fn op_7XKK_add_vx_kk(&mut self, vi: usize, kk: u8) {
        let result = self.vreg[vi].wrapping_add(kk);
        // or let result = self.vreg[vi] as u16 + kk as u16; // ?
        self.vreg[vi] = result;

    }

    // 8XY0 - LD Vx, Vy
    // Set Vx = Vy
    fn op_8XY0_ld_vx_vy(&mut self, vi: usize, vj: usize) {
        self.vreg[vi] = self.vreg[vj];
    }

    // 8XY1 - OR Vx, Vy
    fn op_8XY1_or_vx_vy(&mut self, vi: usize, vj: usize) {
        self.vreg[vi] |= self.vreg[vj];
    }

    // 8XY2 - AND Vx, Vy
    fn op_8XY2_and_vx_vy(&mut self, vi: usize, vj: usize) {
        self.vreg[vi] &= self.vreg[vj];
    }

    // 8XY3 - XOR Vx, Vy
    fn op_8XY3_xor_vx_vy(&mut self, vi: usize, vj: usize) {
        self.vreg[vi] ^= self.vreg[vj];
    }

    // 8XY4 - ADD Vx, Vy
    // Sets VF to 1 if the result is > 0b8 (255), else 0.
    // The lowest 8 bits of the result are stored in Vx.
    fn op_8XY4_add_vx_vy(&mut self, vi: usize, vj: usize) {
        // or: let (result, carry) = self.vreg[vi].overflowing_add(self.vreg[vj]);

        let vx = self.vreg[vi] as u16;
        let vy = self.vreg[vj] as u16;
        let result = vx + vy;
        let carry = result > 0xFF;

        self.vreg[vi] = result as u8;
        self.vreg[0x0F] = if carry { 1 } else { 0 };
    }

    // 8XY5 - SUB Vx, Vy
    // Sets Vx = Vx - Vy (lowest 8 bits of result).
    // Sets VF = NOT borrow (ie if Vx > Vy).
    fn op_8XY5_sub_vx_vy(&mut self, vi: usize, vj: usize) {
        // or: let (result, borrow) = self.vreg[vi].overflowing_sub(self.vreg[vj]);

        let result = self.vreg[vi].wrapping_sub(self.vreg[vj]);
        let borrow = self.vreg[vi] <= self.vreg[vj];

        self.vreg[vi] = result;
        self.vreg[0x0F] = if borrow { 0 } else { 1 };
    }

    // 8X06 - Shr Vx {, Vy}
    // Sets Vx = Vx rightshift 1.
    // Sets VF = the dropped bit, ie if the least-significant bit of Vx was 1.
    fn op_8XY6_shr_vx(&mut self, vi: usize) {
        self.vreg[0x0F] = self.vreg[vi] & 1;
        self.vreg[vi] >>= 1;
    }

    // 8XY7 - SUBN Vx, Vy
    // Sets Vx = Vy - Vx (lowest 8 bits of result).
    // Sets VF = NOT borrow (ie if Vy > Vx).
    fn op_8XY7_subn_vx_vy(&mut self, vi: usize, vj: usize) {
        // or: let (result, borrow) = self.vreg[vj].overflowing_sub(self.vreg[vi]);

        let result = self.vreg[vj].wrapping_sub(self.vreg[vi]);
        let borrow = self.vreg[vj] <= self.vreg[vi];

        self.vreg[vi] = result;
        self.vreg[0x0F] = if borrow { 0 } else { 1 };
    }

    // 8X0E - SHL Vx {, Vy}
    // Sets Vx = Vx leftshift 1.
    // Sets VF = the dropped bit, ide if the most-significant bit of Vx was 1.
    fn op_8XYE_shl_vx(&mut self, vi: usize) {
        self.vreg[0x0F] = (self.vreg[vi] >> 7) & 1;
        // or self.vreg[0x0F] = (self.vreg[vi] & 0b10000000) >> 7;
        self.vreg[vi] <<= 1;
    }

    // 9XY0 - SNE Vx, Vy
    // Skip next instruction if Vx != Vy.
    fn op_9XY0_sne_vx_vy(&mut self, vi: usize, vj: usize) {
        self.skip_if(self.vreg[vi] != self.vreg[vj]);
    }

    // ANNN - LD I, NNN
    // Set I = NNN (address)
    fn op_ANNN_ld_i_nnn(&mut self, nnn: usize) {
        self.ireg = nnn;
    }

    // BNNN - JMP V0, NNN
    // Set PC = V0 + NNN (address)
    fn op_BNNN_jmp_v0_nnn(&mut self, nnn: usize) {
        self.program_counter = (self.vreg[0] as usize) + nnn;
    }

    // CXKK - RND Vx, KK
    // Set Vx = rand() AND KK
    fn op_CXKK_ld_vx_rand_and_kk(&mut self, vi: usize, kk: u8) {
        let rng: u8 = rand::thread_rng().gen();
        self.vreg[vi] = rng & kk;
    }

    // DXYN - DRW Vx, Vy, N
    // Starting at address stored in I, reads N bytes from memory,
    // and draws them as sprites on screen at (Vx, Vy) (wrapping).
    // Sets VF = if pixels are erased.
    fn op_DXYN_drw(&mut self, vi: usize, vj: usize, n: usize) {
        let x_coord = self.vreg[vi] as usize;
        let y_coord = self.vreg[vj] as usize;
        let mut flipped = false;

        for byte in 0..n {
            let y = (y_coord + byte) % SCREEN_HEIGHT;
            let pixels = self.ram[self.ireg + byte];

            for bit in 0..8 {
                let x = (x_coord + bit) % SCREEN_WIDTH;
                let index = y * SCREEN_WIDTH + x;

                let color = pixels & (0b1000_0000 >> bit);

                // v1
                if color != 0 {
                    flipped |= self.screen[index];
                    self.screen[index] ^= true; // (Set VF later at end)
                }

                // or v2...?
                // self.vreg[0x0F] |= color * self.screen[index]; // Set VF by OR-ing with color * pixel?
                // self.screen[index] ^= color; // Draw pixel
            }
        }

        // v1
        self.vreg[0x0F] = if flipped { 1 } else { 0 };

        self.screen_changed = true;
    }

    // EX9E - SKP Vx
    // Skip next instruction if key with value stored in Vx is pressed.
    fn op_EX9E_skp_vx(&mut self, vi: usize) {
        self.skip_if(self.keys[self.vreg[vi] as usize]);
    }

    // EXA1 - SKNP
    // Skip next instruction if key with the value stored in Vx is not pressed.
    fn op_EXA1_sknp_vx(&mut self, vi: usize) {
        self.skip_if(!self.keys[self.vreg[vi] as usize]);
    }

    // FX07 - LD Vx, DT
    // Set Vx = delay timer.
    fn op_FX07_ld_vx_dt(&mut self, vi: usize) {
        self.vreg[vi] = self.delay_timer;
    }

    // FX0A - LD Vx, Key
    // Wait for a key press, and then store the pressed key value in Vx.
    fn op_FX0A_ld_vx_key(&mut self, vi: usize) {
        let mut pressed = false;

        for key in 0..self.keys.len() {
            if self.keys[key] {
                self.vreg[vi] = key as u8;
                pressed = true;
                break;
            }
        }

        // Block/wait if no key pressed by repeating the previous opcode.
        // (Don't infinite while loop because we must not block new input.)
        if !pressed {
            self.program_counter -= 2;
        }
    }

    // FX15 - LD DT, Vx
    // Set delay timer = VX.
    fn op_FX15_ld_dt_vx(&mut self, vi: usize) {
        self.delay_timer = self.vreg[vi];
    }

    // FX18 - LD ST, Vx
    // Set sound timer = Vx.
    fn op_FX18_ld_st_vx(&mut self, vi: usize) {
        self.sound_timer = self.vreg[vi];
    }

    // FX1E - ADD I, Vx
    // Set I = I + VX. Does NOT set VF.
    fn op_FX1E_add_i_vx(&mut self, vi: usize) {
        self.ireg += self.vreg[vi] as usize;
    }

    // FX29 - LD F, Vx
    // Set I = address of sprite for digit Vx (font).
    fn op_FX29_ld_d_vx(&mut self, vi: usize) {
        self.ireg = (self.vreg[vi] as usize) * 5; // each font occupies 5 bytes
    }

    // BCD = Binary-Coded Decimal
    fn easy_to_read_bcd(x: f32) -> [u8; 3] {
        let hundreds = (x / 100.0) as u8;
        let tens = ((x / 10.0) % 10.0) as u8;
        let ones = (x % 10.0) as u8;
        [hundreds, tens, ones]
    }

    // FX33 - LD B, Vx
    // The interpreter takes the decimal value of Vx, and stores each digit in I.
    fn op_FX33_ld_b_vx(&mut self, vi: usize) {
        let vx = self.vreg[vi] as f32;

        let [hundreds, tens, ones] = Emulator::easy_to_read_bcd(vx);

        self.ram[self.ireg as usize] = hundreds;
        self.ram[(self.ireg + 1) as usize] = tens;
        self.ram[(self.ireg + 2) as usize] = ones;
    }

    // FX55 - LD [I], Vx
    // Load V0 through VX into memory at address stored in I register.
    fn op_FX55_ld_i_vx(&mut self, n: usize) {
        let start_address = self.ireg;
        for i in 0..=n {
            self.ram[start_address + i] = self.vreg[i];
        }
    }

    // FX65 - LD Vx, [I]
    // Load values from memory starting at address I into registers V0 thru Vx
    fn op_FX65_ld_vx_i(&mut self, n: usize) {
        let start_address = self.ireg;
        for i in 0..=n {
            self.vreg[i] = self.ram[start_address + i]
        }
    }
}
