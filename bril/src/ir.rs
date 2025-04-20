use std::{iter, ops::Range};

pub type StringIdx = u32;

pub const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Variable(pub u32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct LabelIdx(pub u32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct FunctionIdx(pub u32);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Instruction {
    Add(Variable, Variable),
    Mul(Variable, Variable),
    Sub(Variable, Variable),
    Div(Variable, Variable),

    Eq(Variable, Variable),
    Lt(Variable, Variable),
    Gt(Variable, Variable),
    Le(Variable, Variable),
    Ge(Variable, Variable),

    Not(Variable),
    And(Variable, Variable),
    Or(Variable, Variable),

    Jmp(LabelIdx),
    Br(Variable, LabelIdx, LabelIdx),
    Call(FunctionIdx, Box<[Variable]>),
    Ret(Option<Variable>),

    Id(Variable),
    Print(Box<[Variable]>),
    Nop,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
struct FunctionInternal {
    start: usize,
    name: StringIdx,
    parameters: Vec<Variable>,
}

pub struct Function<'a> {
    /// The subarray of instructions corresponding to this function.
    pub instruction_range: Range<usize>,
    pub name: StringIdx,
    pub parameters: &'a [Variable],
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    strings: Vec<String>,
    labels: Vec<(usize, StringIdx)>,
    functions: Vec<FunctionInternal>,
}

impl Program {
    pub fn functions(&self) -> impl Iterator<Item = Function> {
        let ends = self
            .functions
            .iter()
            .skip(1)
            .map(|function| function.start)
            .chain(iter::once(self.functions.len()));
        self.functions
            .iter()
            .zip(ends)
            .map(|(function, end)| Function {
                instruction_range: function.start..end,
                name: function.name,
                parameters: &function.parameters,
            })
    }

    pub fn add_label(&mut self, string: impl Into<String>) -> LabelIdx {
        self.strings.push(string.into());
        let string_idx = (self.strings.len() - 1) as StringIdx;
        let current_position = self.instructions.len();
        self.labels.push((current_position, string_idx));
        LabelIdx((self.labels.len() - 1) as u32)
    }

    pub fn get_string(&self, idx: StringIdx) -> &str {
        &self.strings[idx as usize]
    }
}
