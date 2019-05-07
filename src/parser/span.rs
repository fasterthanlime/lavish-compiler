use super::Source;
use nom::{
    Compare, CompareResult, FindSubstring, InputIter, InputLength, InputTake, Slice,
    UnspecializedInput,
};
use std::ops::RangeFrom;
use std::rc::Rc;

#[derive(Clone)]
pub struct Span {
    pub source: Rc<Source>,
    pub offset: usize,
    pub len: usize,
}

impl Span {
    pub fn chars(&self) -> SpanIterElem {
        SpanIterElem {
            span: self.clone(),
            offset: 0,
        }
    }

    pub fn char_indices(&self) -> SpanIter {
        SpanIter {
            span: self.clone(),
            offset: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.source.input[self.offset..self.offset + self.len].len()
    }
}

impl PartialEq for Span {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.source, &other.source)
            && self.offset == other.offset
            && self.len == other.len
    }
}
impl Eq for Span {}

impl InputLength for Span {
    fn input_len(&self) -> usize {
        self.len
    }
}

pub struct SpanIterElem {
    span: Span,
    offset: usize,
}

impl Iterator for SpanIterElem {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(c) = self
            .span
            .source
            .input
            .chars()
            .nth(self.span.offset + self.offset)
        {
            self.offset += 1;
            Some(c)
        } else {
            None
        }
    }
}

pub struct SpanIter {
    span: Span,
    offset: usize,
}

impl Iterator for SpanIter {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(c) = self
            .span
            .source
            .input
            .char_indices()
            .nth(self.span.offset + self.offset)
        {
            self.offset += 1;
            Some(c)
        } else {
            None
        }
    }
}

impl InputIter for Span {
    type Item = char;
    type RawItem = char;
    type Iter = SpanIter;
    type IterElem = SpanIterElem;

    #[inline]
    fn iter_indices(&self) -> Self::Iter {
        self.char_indices()
    }
    #[inline]
    fn iter_elements(&self) -> Self::IterElem {
        self.chars()
    }
    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::RawItem) -> bool,
    {
        for (o, c) in self.char_indices() {
            if predicate(c) {
                return Some(o);
            }
        }
        None
    }
    #[inline]
    fn slice_index(&self, count: usize) -> Option<usize> {
        let mut cnt = 0;
        for (index, _) in self.char_indices() {
            if cnt == count {
                return Some(index);
            }
            cnt += 1;
        }
        if cnt == count {
            return Some(self.len());
        }
        None
    }
}

impl InputTake for Span {
    fn take(&self, count: usize) -> Self {
        Self {
            source: self.source.clone(),
            offset: self.offset,
            len: self.len - count,
        }
    }
    fn take_split(&self, count: usize) -> (Self, Self) {
        (
            Self {
                source: self.source.clone(),
                offset: self.offset + count,
                len: self.len - count,
            },
            Self {
                source: self.source.clone(),
                offset: self.offset,
                len: self.len - count,
            },
        )
    }
}

impl UnspecializedInput for Span {}

impl Compare<&str> for Span {
    #[inline(always)]
    fn compare(&self, t: &str) -> CompareResult {
        let pos = self.chars().zip(t.chars()).position(|(a, b)| a != b);

        match pos {
            Some(_) => CompareResult::Error,
            None => {
                if self.len() >= t.len() {
                    CompareResult::Ok
                } else {
                    CompareResult::Incomplete
                }
            }
        }
    }

    //FIXME: this version is too simple and does not use the current locale
    #[inline(always)]
    fn compare_no_case(&self, t: &str) -> CompareResult {
        let pos = self
            .chars()
            .zip(t.chars())
            .position(|(a, b)| a.to_lowercase().zip(b.to_lowercase()).any(|(a, b)| a != b));

        match pos {
            Some(_) => CompareResult::Error,
            None => {
                if self.len() >= t.len() {
                    CompareResult::Ok
                } else {
                    CompareResult::Incomplete
                }
            }
        }
    }
}

impl Slice<RangeFrom<usize>> for Span {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        unimplemented!();
    }
}

impl Into<String> for Span {
    fn into(self) -> String {
        self.source.input[self.offset..self.offset + self.len].into()
    }
}

impl FindSubstring<&str> for Span {
    fn find_substring(&self, substr: &str) -> Option<usize> {
        unimplemented!();
    }
}
