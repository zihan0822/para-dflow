// Copyright (C) 2025 Zihan Li and Ethan Uppal.

use std::{iter, ops::Range};

pub const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Variable(pub u32, pub Type);
/// A global index into the label array.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct LabelIdx(pub u32);
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct FunctionIdx(pub u32);
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct StringIdx(pub u32);

macro_rules! impl_undef {
    ($($ty: ident),+) => {
        $(impl $ty {
            pub const UNDEF: $ty = $ty(NO_INDEX);
        })+
    }
}

impl_undef!(LabelIdx, FunctionIdx, StringIdx);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(val) => write!(formatter, "{val:?}")?,
            Self::Int(val) => write!(formatter, "{val:?}")?,
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Type {
    Int,
    Bool,
}

impl std::fmt::Display for Type {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool => formatter.write_str("bool")?,
            Self::Int => formatter.write_str("int")?,
        }
        Ok(())
    }
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
    pub(crate) labels: Vec<LabelIdx>,
    pub(crate) return_type: Option<Type>,
}

#[derive(Debug, Copy, Clone)]
pub struct Label<'a> {
    /// offset has different interpretation in different context
    /// in Function, offset is relative to the start of function's instruction
    /// buffer in Program, offset is relative to the start of global
    /// instruction buffer
    pub offset: usize,
    pub name: &'a str,
    pub idx: LabelIdx,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FunctionPrototype {
    pub name: String,
    pub parameters: Vec<Variable>,
    pub return_type: Option<Type>,
}

pub struct Function<'a> {
    /// The subarray of instructions corresponding to this function.
    pub instructions: &'a [Instruction],
    pub name: &'a str,
    pub parameters: &'a [Variable],
    /// sorted in ascending order by offset
    pub labels: Vec<Label<'a>>,
    pub return_type: Option<Type>,
}

impl Function<'_> {
    pub fn prototype(&self) -> FunctionPrototype {
        FunctionPrototype {
            name: self.name.to_string(),
            parameters: self.parameters.to_vec(),
            return_type: self.return_type,
        }
    }
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
        (0..self.functions.len())
            .map(|idx| self.get_function(FunctionIdx(idx as u32)))
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

    pub fn get_function(&self, idx: FunctionIdx) -> Function {
        let function = &self.functions[idx.0 as usize];
        let start = function.range.start;
        let mut labels: Vec<_> = function
            .labels
            .iter()
            .map(|label_idx| Label {
                offset: self.get_label_offset(*label_idx) - start,
                name: self.get_label_name(*label_idx),
                idx: *label_idx,
            })
            .collect();
        labels.sort_by_key(|label| label.offset);
        Function {
            instructions: &self.instructions[function.range.clone()],
            name: self.get_string(function.name),
            parameters: &function.parameters,
            return_type: function.return_type,
            labels,
        }
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

/// See [`Function::items_iter`].
pub enum FunctionItem<'a> {
    Instruction(&'a Instruction),
    Label(LabelIdx),
}

impl Function<'_> {
    /// An iterator over the labels and instructions in this function
    /// interleaved in the correct order.
    pub fn items_iter(&self) -> impl Iterator<Item = FunctionItem<'_>> {
        let mut labels = self.labels.iter().enumerate().peekable();
        let mut instructions = self.instructions.iter().enumerate().peekable();

        iter::from_fn(move || {
            if let Some(&(i, _)) = instructions.peek() {
                if labels
                    .peek()
                    .map(|(_, label)| label.offset == i)
                    .unwrap_or(false)
                {
                    Some(FunctionItem::Label(labels.next()?.1.idx))
                } else {
                    Some(FunctionItem::Instruction(instructions.next()?.1))
                }
            } else {
                Some(FunctionItem::Label(labels.next()?.1.idx))
            }
        })
    }
}
