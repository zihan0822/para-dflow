use std::ops::Range;

pub const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Variable(pub u32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct LabelIdx(pub u32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct FunctionIdx(pub u32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct StringIdx(pub u32);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Instruction {
    Add(Variable, Variable, Variable),
    Mul(Variable, Variable, Variable),
    Sub(Variable, Variable, Variable),
    Div(Variable, Variable, Variable),

    Eq(Variable, Variable, Variable),
    Lt(Variable, Variable, Variable),
    Gt(Variable, Variable, Variable),
    Le(Variable, Variable, Variable),
    Ge(Variable, Variable, Variable),

    Not(Variable, Variable),
    And(Variable, Variable, Variable),
    Or(Variable, Variable, Variable),

    Jmp(LabelIdx),
    Br(Variable, LabelIdx, LabelIdx),
    Call(Option<Variable>, FunctionIdx, Box<[Variable]>),
    Ret(Option<Variable>),

    Const(Variable, Value),
    Id(Variable, Variable),
    Print(Box<[Variable]>),
    Nop,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct FunctionInternal {
    pub(crate) range: Range<usize>,
    pub(crate) name: StringIdx,
    pub(crate) parameters: Vec<Variable>,
}

pub struct Function<'a> {
    /// The subarray of instructions corresponding to this function.
    pub instructions: &'a [Instruction],
    pub name: &'a str,
    pub parameters: &'a [Variable],
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    functions: Vec<FunctionInternal>,
    strings: Vec<String>,
    labels: Vec<(usize, StringIdx)>,
}

impl Program {
    pub fn functions(&self) -> impl Iterator<Item = Function> {
        self.functions.iter().map(|function| Function {
            instructions: &self.instructions[function.range.clone()],
            name: self.get_string(function.name),
            parameters: &function.parameters,
        })
    }

    pub fn add_label(&mut self, string: impl Into<String>) -> LabelIdx {
        self.strings.push(string.into());
        let string_idx = StringIdx((self.strings.len() - 1) as u32);
        let current_position = self.instructions.len();
        self.labels.push((current_position, string_idx));
        LabelIdx((self.labels.len() - 1) as u32)
    }

    pub fn get_label_offset(&self, idx: LabelIdx) -> usize {
        self.labels[idx.0 as usize].0
    }
    pub fn get_label_name(&self, idx: LabelIdx) -> &str {
        self.get_string(self.labels[idx.0 as usize].1)
    }

    pub fn add_string(&mut self, string: impl Into<String>) -> StringIdx {
        self.strings.push(string.into());
        StringIdx((self.strings.len() - 1) as u32)
    }

    pub fn get_string(&self, idx: StringIdx) -> &str {
        &self.strings[idx.0 as usize]
    }

    pub fn find_function_symbol(&self, name: &str) -> Option<FunctionIdx> {
        self.functions()
            .position(|function| function.name == name)
            .map(|idx| FunctionIdx(idx as u32))
    }

    pub(crate) fn add_function(
        &mut self,
        function: FunctionInternal,
    ) -> FunctionIdx {
        self.functions.push(function);
        FunctionIdx((self.functions.len() - 1) as u32)
    }
}
