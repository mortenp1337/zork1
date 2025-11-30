//! Z-Machine opcode execution

use crate::zmachine::{ZMachine, Instruction, Opcode};

impl ZMachine {
    /// Execute an instruction
    pub fn execute_instruction(&mut self, instr: &Instruction) -> Result<(), String> {
        match &instr.opcode {
            Opcode::ZeroOp(op) => self.execute_0op(*op),
            Opcode::OneOp(op) => self.execute_1op(*op, &instr.operands),
            Opcode::TwoOp(op) => self.execute_2op(*op, &instr.operands),
            Opcode::Var(op, count) => self.execute_var(*op, *count, &instr.operands),
            Opcode::Extended(op) => self.execute_ext(*op, &instr.operands),
        }
    }
    
    /// Execute a 0OP instruction
    fn execute_0op(&mut self, op: u8) -> Result<(), String> {
        match op {
            0x00 => {
                // rtrue - return true (1)
                self.return_from_routine(1);
            }
            0x01 => {
                // rfalse - return false (0)
                self.return_from_routine(0);
            }
            0x02 => {
                // print - print literal string
                let len = self.print_zstring(self.pc);
                self.pc += len;
            }
            0x03 => {
                // print_ret - print and return true
                let len = self.print_zstring(self.pc);
                self.pc += len;
                self.print_newline();
                self.return_from_routine(1);
            }
            0x04 => {
                // nop
            }
            0x05 => {
                // save (v3: branch on success)
                // Not implemented - branch on failure
                self.branch(false);
            }
            0x06 => {
                // restore (v3: branch on success)
                // Not implemented - branch on failure
                self.branch(false);
            }
            0x07 => {
                // restart
                self.memory.restart();
                let initial_pc = self.memory.read_word(0x06) as usize;
                self.pc = initial_pc;
                self.frames.clear();
                self.frames.push(crate::zmachine::Frame {
                    return_pc: 0,
                    locals: Vec::new(),
                    stack: Vec::new(),
                    store_var: None,
                });
            }
            0x08 => {
                // ret_popped - return with value popped from stack
                let value = self.frames.last_mut().unwrap().stack.pop().unwrap_or(0);
                self.return_from_routine(value);
            }
            0x09 => {
                // pop (v3) / catch (v5+)
                self.frames.last_mut().unwrap().stack.pop();
            }
            0x0A => {
                // quit
                self.running = false;
            }
            0x0B => {
                // new_line
                self.print_newline();
            }
            0x0C => {
                // show_status (v3)
                // Display status line - implementation dependent
            }
            0x0D => {
                // verify - verify story file checksum
                // Always succeed for now
                self.branch(true);
            }
            _ => {
                return Err(format!("Unknown 0OP opcode: {:02X}", op));
            }
        }
        Ok(())
    }
    
    /// Execute a 1OP instruction
    fn execute_1op(&mut self, op: u8, operands: &[u16]) -> Result<(), String> {
        let arg = operands.get(0).copied().unwrap_or(0);
        
        match op {
            0x00 => {
                // jz - jump if zero
                self.branch(arg == 0);
            }
            0x01 => {
                // get_sibling
                let sibling = self.get_object_sibling(arg);
                self.store_result(sibling);
                self.branch(sibling != 0);
            }
            0x02 => {
                // get_child
                let child = self.get_object_child(arg);
                self.store_result(child);
                self.branch(child != 0);
            }
            0x03 => {
                // get_parent
                let parent = self.get_object_parent(arg);
                self.store_result(parent);
            }
            0x04 => {
                // get_prop_len
                let len = if arg == 0 {
                    0
                } else {
                    let size_byte = self.memory.read_byte(arg as usize - 1);
                    ((size_byte >> 5) + 1) as u16
                };
                self.store_result(len);
            }
            0x05 => {
                // inc
                let var_num = arg as u8;
                let value = self.get_variable(var_num);
                self.set_variable(var_num, value.wrapping_add(1));
            }
            0x06 => {
                // dec
                let var_num = arg as u8;
                let value = self.get_variable(var_num);
                self.set_variable(var_num, value.wrapping_sub(1));
            }
            0x07 => {
                // print_addr
                self.print_zstring(arg as usize);
            }
            0x08 => {
                // call_1s (v4+) - not in v3
                return Err("call_1s not supported in v3".to_string());
            }
            0x09 => {
                // remove_obj
                self.remove_object(arg);
            }
            0x0A => {
                // print_obj
                let name = self.get_object_name(arg);
                self.print(&name);
            }
            0x0B => {
                // ret
                self.return_from_routine(arg);
            }
            0x0C => {
                // jump
                let offset = arg as i16;
                self.pc = ((self.pc as i32) + (offset as i32) - 2) as usize;
            }
            0x0D => {
                // print_paddr
                let addr = (arg as usize) * 2;
                self.print_zstring(addr);
            }
            0x0E => {
                // load
                let var_num = arg as u8;
                let value = if var_num == 0 {
                    // Peek stack, don't pop
                    self.frames.last().unwrap().stack.last().copied().unwrap_or(0)
                } else {
                    self.get_variable(var_num)
                };
                self.store_result(value);
            }
            0x0F => {
                // not (v3) / call_1n (v5+)
                self.store_result(!arg);
            }
            _ => {
                return Err(format!("Unknown 1OP opcode: {:02X}", op));
            }
        }
        Ok(())
    }
    
    /// Execute a 2OP instruction
    fn execute_2op(&mut self, op: u8, operands: &[u16]) -> Result<(), String> {
        let arg1 = operands.get(0).copied().unwrap_or(0);
        let arg2 = operands.get(1).copied().unwrap_or(0);
        
        match op {
            0x00 => {
                // 2OP:0 doesn't exist - likely a decoding error or nop
                // This can happen in some edge cases
            }
            0x01 => {
                // je - jump if equal
                let equal = operands.iter().skip(1).any(|&x| x == arg1);
                self.branch(equal);
            }
            0x02 => {
                // jl - jump if less than (signed)
                self.branch((arg1 as i16) < (arg2 as i16));
            }
            0x03 => {
                // jg - jump if greater than (signed)
                self.branch((arg1 as i16) > (arg2 as i16));
            }
            0x04 => {
                // dec_chk - decrement and branch if less than (signed)
                let var_num = arg1 as u8;
                let value = (self.get_variable(var_num) as i16).wrapping_sub(1);
                self.set_variable(var_num, value as u16);
                self.branch(value < arg2 as i16);
            }
            0x05 => {
                // inc_chk - increment and branch if greater than (signed)
                let var_num = arg1 as u8;
                let value = (self.get_variable(var_num) as i16).wrapping_add(1);
                self.set_variable(var_num, value as u16);
                self.branch(value > arg2 as i16);
            }
            0x06 => {
                // jin - jump if parent
                let parent = self.get_object_parent(arg1);
                self.branch(parent == arg2);
            }
            0x07 => {
                // test - bitwise AND test
                self.branch((arg1 & arg2) == arg2);
            }
            0x08 => {
                // or
                self.store_result(arg1 | arg2);
            }
            0x09 => {
                // and
                self.store_result(arg1 & arg2);
            }
            0x0A => {
                // test_attr
                self.branch(self.get_object_attr(arg1, arg2));
            }
            0x0B => {
                // set_attr
                self.set_object_attr(arg1, arg2, true);
            }
            0x0C => {
                // clear_attr
                self.set_object_attr(arg1, arg2, false);
            }
            0x0D => {
                // store
                let var_num = arg1 as u8;
                self.set_variable(var_num, arg2);
            }
            0x0E => {
                // insert_obj
                self.insert_object(arg1, arg2);
            }
            0x0F => {
                // loadw
                let addr = arg1.wrapping_add(arg2.wrapping_mul(2));
                let value = self.memory.read_word(addr as usize);
                self.store_result(value);
            }
            0x10 => {
                // loadb
                let addr = arg1.wrapping_add(arg2);
                let value = self.memory.read_byte(addr as usize) as u16;
                self.store_result(value);
            }
            0x11 => {
                // get_prop
                let value = self.get_property(arg1, arg2);
                self.store_result(value);
            }
            0x12 => {
                // get_prop_addr
                let (_, addr, _) = self.find_property(arg1, arg2);
                self.store_result(addr as u16);
            }
            0x13 => {
                // get_next_prop
                let next = self.get_next_property(arg1, arg2);
                self.store_result(next);
            }
            0x14 => {
                // add
                let result = (arg1 as i16).wrapping_add(arg2 as i16);
                self.store_result(result as u16);
            }
            0x15 => {
                // sub
                let result = (arg1 as i16).wrapping_sub(arg2 as i16);
                self.store_result(result as u16);
            }
            0x16 => {
                // mul
                let result = (arg1 as i16).wrapping_mul(arg2 as i16);
                self.store_result(result as u16);
            }
            0x17 => {
                // div
                if arg2 == 0 {
                    return Err("Division by zero".to_string());
                }
                let result = (arg1 as i16) / (arg2 as i16);
                self.store_result(result as u16);
            }
            0x18 => {
                // mod
                if arg2 == 0 {
                    return Err("Division by zero".to_string());
                }
                let result = (arg1 as i16) % (arg2 as i16);
                self.store_result(result as u16);
            }
            _ => {
                return Err(format!("Unknown 2OP opcode: {:02X}", op));
            }
        }
        Ok(())
    }
    
    /// Execute a VAR instruction
    fn execute_var(&mut self, op: u8, _count: u8, operands: &[u16]) -> Result<(), String> {
        match op {
            0x00 => {
                // call / call_vs
                if operands.is_empty() {
                    return Err("call requires at least 1 operand".to_string());
                }
                let routine_addr = operands[0];
                let args: Vec<u16> = operands.iter().skip(1).copied().collect();
                let store_var = self.read_byte();
                self.call_routine(routine_addr, &args, Some(store_var));
            }
            0x01 => {
                // storew
                let array = operands.get(0).copied().unwrap_or(0);
                let index = operands.get(1).copied().unwrap_or(0);
                let value = operands.get(2).copied().unwrap_or(0);
                let addr = array.wrapping_add(index.wrapping_mul(2));
                self.memory.write_word(addr as usize, value);
            }
            0x02 => {
                // storeb
                let array = operands.get(0).copied().unwrap_or(0);
                let index = operands.get(1).copied().unwrap_or(0);
                let value = operands.get(2).copied().unwrap_or(0);
                let addr = array.wrapping_add(index);
                self.memory.write_byte(addr as usize, value as u8);
            }
            0x03 => {
                // put_prop
                let obj = operands.get(0).copied().unwrap_or(0);
                let prop = operands.get(1).copied().unwrap_or(0);
                let value = operands.get(2).copied().unwrap_or(0);
                self.set_property(obj, prop, value);
            }
            0x04 => {
                // sread (v3) / aread (v5+)
                let text_buffer = operands.get(0).copied().unwrap_or(0) as usize;
                let parse_buffer = operands.get(1).copied().unwrap_or(0) as usize;
                self.read_line(text_buffer, parse_buffer);
            }
            0x05 => {
                // print_char
                let char_code = operands.get(0).copied().unwrap_or(0);
                if char_code >= 32 && char_code < 127 {
                    self.print(&(char_code as u8 as char).to_string());
                } else if char_code == 13 {
                    self.print_newline();
                }
            }
            0x06 => {
                // print_num
                let num = operands.get(0).copied().unwrap_or(0) as i16;
                self.print(&num.to_string());
            }
            0x07 => {
                // random
                let range = operands.get(0).copied().unwrap_or(0) as i16;
                let result = self.random(range);
                self.store_result(result);
            }
            0x08 => {
                // push
                let value = operands.get(0).copied().unwrap_or(0);
                self.frames.last_mut().unwrap().stack.push(value);
            }
            0x09 => {
                // pull
                let var_num = operands.get(0).copied().unwrap_or(0) as u8;
                let value = self.frames.last_mut().unwrap().stack.pop().unwrap_or(0);
                self.set_variable(var_num, value);
            }
            0x0A => {
                // split_window (v3+)
                // Screen splitting - not fully implemented
            }
            0x0B => {
                // set_window (v3+)
                // Window selection - not fully implemented
            }
            0x13 => {
                // output_stream
                // Stream control - basic implementation
            }
            0x14 => {
                // input_stream
                // Input stream - not implemented
            }
            0x15 => {
                // sound_effect
                // Sound effects - not implemented
            }
            _ => {
                return Err(format!("Unknown VAR opcode: {:02X}", op));
            }
        }
        Ok(())
    }
    
    /// Execute an extended instruction
    fn execute_ext(&mut self, op: u8, _operands: &[u16]) -> Result<(), String> {
        // Extended opcodes are not common in v3
        Err(format!("Extended opcode not implemented: {:02X}", op))
    }
}

// Re-export Frame for use in opcodes.rs
#[derive(Clone, Debug)]
pub struct Frame {
    pub return_pc: usize,
    pub locals: Vec<u16>,
    pub stack: Vec<u16>,
    pub store_var: Option<u8>,
}
