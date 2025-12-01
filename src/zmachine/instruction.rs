//! Z-Machine instruction definitions

/// An instruction opcode
#[derive(Debug, Clone)]
pub enum Opcode {
    /// 0OP instruction
    ZeroOp(u8),
    /// 1OP instruction
    OneOp(u8),
    /// 2OP instruction
    TwoOp(u8),
    /// VAR instruction (op_count: 0=VAR, 2=2OP)
    Var(u8, u8),
    /// Extended instruction
    Extended(u8),
}

/// Operand type
#[derive(Debug, Clone, Copy)]
pub enum OperandType {
    /// Large constant (2 bytes)
    LargeConstant,
    /// Small constant (1 byte)
    SmallConstant,
    /// Variable (1 byte)
    Variable,
    /// Omitted operand
    Omitted,
}

/// A decoded instruction
#[derive(Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<u16>,
}
