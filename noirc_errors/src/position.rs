use codespan::{Span as ByteSpan,ByteIndex};

#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Debug,Copy, Clone)]
pub struct Position{
    file_id : usize,
    line : usize,
    column : usize,
    idx : usize,
}

impl Default for Position {
    fn default() -> Self{
        Position{
            file_id: 0,
            line: 0,
            column: 0,
            idx: 0
        }
    }
}

impl Position {
    pub fn default_from(file_id : usize) -> Self{
        Position{
            file_id,
            line: 0,
            column: 0,
            idx: 0
        }
    }
    pub fn new_line(&mut self) {
        self.line +=1;
        self.column = 0;
        self.idx += 1;
    }
    pub fn right_shift(&mut self) {
        self.column +=1;
        self.idx += 1;
    }

    pub fn mark(&self) -> Position {
        self.clone()
    }
    pub fn forward(self) -> Position{
        self.forward_by(1)
    }
    pub fn backward(self) -> Position{
        self.backward_by(1)
    }
    
    pub fn into_span(self) -> Span{
        Span{
            file_id : self.file_id,
            start : self,
            end : self,
        }
    }

    pub fn backward_by(self, amount : usize) -> Position {
        Position {
            file_id : self.file_id,
            line : self.line,
            column : self.column - amount,
            idx : self.idx - amount
        }
    }
    pub fn forward_by(self, amount : usize) -> Position {
        Position {
            file_id : self.file_id,
            line : self.line,
            column : self.column + amount,
            idx : self.idx + amount
        }
    }
    pub fn to_byte_index(self) -> ByteIndex {
        ByteIndex((self.idx -1) as u32)
    }

}

#[derive(PartialOrd, Eq, Ord, Hash, Debug, Clone)]
pub struct Spanned<T> {
    pub contents : T,
    span : Span,
}

/// This is important for tests. Two Spanned objects are equal iff their content is equal
/// They may not have the same span. use into_span to test for Span being equal specifically
impl<T : std::cmp::PartialEq> PartialEq<Spanned<T>> for Spanned<T> {
    fn eq(&self, other: &Spanned<T>) -> bool {
        self.contents == other.contents
    }
}

impl<T> Spanned<T> {
    
    pub fn from_position(start : Position, end : Position, contents : T) -> Spanned<T> {
        Spanned {
            span : Span{
               file_id :start.file_id, start, end
            },contents
        }
    }
    pub fn from(t_span : Span, contents : T) -> Spanned<T> {
        Spanned {
            span : t_span,contents
        }
    }

    pub fn span(&self) -> Span {
        self.span.clone()
    }
}

impl<T> std::borrow::Borrow<T> for Spanned<T> {
    fn borrow(&self) -> &T {
        &self.contents
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Copy, Clone)]
pub struct Span {
    file_id : usize,
    pub start : Position,
    pub end : Position,
}

impl Default for Span {
    fn default() -> Self{
        Span{
            file_id : 0,
            start: Position::default(),
            end: Position::default(),
        }
    }
}

impl Span {
    pub fn merge(self, other : Span) -> Span {
        
        assert_eq!(self.file_id, other.file_id);
        let file_id = self.file_id;

        use std::cmp::{max, min};

        let start = min(self.start, other.start);
        let end = max(self.end, other.end);
        Span{file_id,start,end}
    }
    pub fn to_byte_span(self) -> ByteSpan {
        ByteSpan::from(self.start.to_byte_index() .. self.end.to_byte_index())
    }
}