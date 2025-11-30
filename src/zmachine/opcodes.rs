//! Z-machine opcode definitions
//!
//! This module contains opcode constants and documentation.

// 2OP opcodes (count 0x01-0x1F)
pub const OP2_JE: u8 = 0x01;
pub const OP2_JL: u8 = 0x02;
pub const OP2_JG: u8 = 0x03;
pub const OP2_DEC_CHK: u8 = 0x04;
pub const OP2_INC_CHK: u8 = 0x05;
pub const OP2_JIN: u8 = 0x06;
pub const OP2_TEST: u8 = 0x07;
pub const OP2_OR: u8 = 0x08;
pub const OP2_AND: u8 = 0x09;
pub const OP2_TEST_ATTR: u8 = 0x0A;
pub const OP2_SET_ATTR: u8 = 0x0B;
pub const OP2_CLEAR_ATTR: u8 = 0x0C;
pub const OP2_STORE: u8 = 0x0D;
pub const OP2_INSERT_OBJ: u8 = 0x0E;
pub const OP2_LOADW: u8 = 0x0F;
pub const OP2_LOADB: u8 = 0x10;
pub const OP2_GET_PROP: u8 = 0x11;
pub const OP2_GET_PROP_ADDR: u8 = 0x12;
pub const OP2_GET_NEXT_PROP: u8 = 0x13;
pub const OP2_ADD: u8 = 0x14;
pub const OP2_SUB: u8 = 0x15;
pub const OP2_MUL: u8 = 0x16;
pub const OP2_DIV: u8 = 0x17;
pub const OP2_MOD: u8 = 0x18;

// 1OP opcodes (base 0x80)
pub const OP1_JZ: u8 = 0x80;
pub const OP1_GET_SIBLING: u8 = 0x81;
pub const OP1_GET_CHILD: u8 = 0x82;
pub const OP1_GET_PARENT: u8 = 0x83;
pub const OP1_GET_PROP_LEN: u8 = 0x84;
pub const OP1_INC: u8 = 0x85;
pub const OP1_DEC: u8 = 0x86;
pub const OP1_PRINT_ADDR: u8 = 0x87;
pub const OP1_REMOVE_OBJ: u8 = 0x89;
pub const OP1_PRINT_OBJ: u8 = 0x8A;
pub const OP1_RET: u8 = 0x8B;
pub const OP1_JUMP: u8 = 0x8C;
pub const OP1_PRINT_PADDR: u8 = 0x8D;
pub const OP1_LOAD: u8 = 0x8E;
pub const OP1_NOT: u8 = 0x8F;

// 0OP opcodes (base 0xB0)
pub const OP0_RTRUE: u8 = 0xB0;
pub const OP0_RFALSE: u8 = 0xB1;
pub const OP0_PRINT: u8 = 0xB2;
pub const OP0_PRINT_RET: u8 = 0xB3;
pub const OP0_NOP: u8 = 0xB4;
pub const OP0_SAVE: u8 = 0xB5;
pub const OP0_RESTORE: u8 = 0xB6;
pub const OP0_RESTART: u8 = 0xB7;
pub const OP0_RET_POPPED: u8 = 0xB8;
pub const OP0_POP: u8 = 0xB9;
pub const OP0_QUIT: u8 = 0xBA;
pub const OP0_NEW_LINE: u8 = 0xBB;
pub const OP0_SHOW_STATUS: u8 = 0xBC;
pub const OP0_VERIFY: u8 = 0xBD;

// VAR opcodes (base 0xE0)
pub const VAR_CALL: u8 = 0xE0;
pub const VAR_STOREW: u8 = 0xE1;
pub const VAR_STOREB: u8 = 0xE2;
pub const VAR_PUT_PROP: u8 = 0xE3;
pub const VAR_SREAD: u8 = 0xE4;
pub const VAR_PRINT_CHAR: u8 = 0xE5;
pub const VAR_PRINT_NUM: u8 = 0xE6;
pub const VAR_RANDOM: u8 = 0xE7;
pub const VAR_PUSH: u8 = 0xE8;
pub const VAR_PULL: u8 = 0xE9;
pub const VAR_SPLIT_WINDOW: u8 = 0xEA;
pub const VAR_SET_WINDOW: u8 = 0xEB;
