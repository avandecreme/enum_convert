//! Custom parsing utilities to replace `syn` dependency.
//!
//! This module provides minimal parsing infrastructure for the derive macros.

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use std::fmt;
use std::iter::Peekable;

/// A parsing error with span information.
#[derive(Debug, Clone)]
pub struct Error {
    span: Span,
    end_span: Option<Span>,
    message: String,
}

impl Error {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            end_span: None,
            message: message.into(),
        }
    }

    /// Create an error that spans from `start` to `end`.
    pub fn new_span_range(start: Span, end: Span, message: impl Into<String>) -> Self {
        Self {
            span: start,
            end_span: Some(end),
            message: message.into(),
        }
    }

    pub fn into_compile_error(self) -> TokenStream {
        let start = self.span;
        let end = self.end_span.unwrap_or(start);

        // Build: compile_error! { "message" }
        // The trick for multi-token spans: set the opening tokens to start span,
        // and the closing group to end span. The compiler will report the error
        // as spanning from start to end.
        let mut tokens = TokenStream::new();

        // compile_error
        let ident = Ident::new("compile_error", start);
        tokens.extend(std::iter::once(TokenTree::Ident(ident)));

        // !
        let mut bang = Punct::new('!', Spacing::Alone);
        bang.set_span(start);
        tokens.extend(std::iter::once(TokenTree::Punct(bang)));

        // { "message" }
        let mut msg = Literal::string(&self.message);
        msg.set_span(start);
        let mut group = Group::new(Delimiter::Brace, std::iter::once(TokenTree::Literal(msg)).collect());
        group.set_span(end);
        tokens.extend(std::iter::once(TokenTree::Group(group)));

        tokens
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A cursor over a token stream for parsing.
pub struct ParseStream<'a> {
    tokens: Peekable<std::vec::IntoIter<TokenTree>>,
    span: Span,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ParseStream<'a> {
    pub fn new(tokens: TokenStream) -> Self {
        let vec: Vec<TokenTree> = tokens.into_iter().collect();
        let span = vec.first().map(|t| t.span()).unwrap_or_else(Span::call_site);
        Self {
            tokens: vec.into_iter().peekable(),
            span,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn span(&self) -> Span {
        self.tokens
            .clone()
            .peek()
            .map(|t| t.span())
            .unwrap_or(self.span)
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.clone().peek().is_none()
    }

    pub fn peek_ident(&mut self) -> Option<&Ident> {
        match self.tokens.peek() {
            Some(TokenTree::Ident(ident)) => Some(ident),
            _ => None,
        }
    }

    pub fn peek_punct(&mut self) -> Option<char> {
        match self.tokens.peek() {
            Some(TokenTree::Punct(p)) => Some(p.as_char()),
            _ => None,
        }
    }

    pub fn peek_literal(&mut self) -> Option<&Literal> {
        match self.tokens.peek() {
            Some(TokenTree::Literal(lit)) => Some(lit),
            _ => None,
        }
    }

    pub fn peek_group(&mut self, delimiter: Delimiter) -> Option<&Group> {
        match self.tokens.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == delimiter => Some(g),
            _ => None,
        }
    }

    pub fn next_token(&mut self) -> Option<TokenTree> {
        self.tokens.next()
    }

    pub fn parse_ident(&mut self) -> Result<Ident> {
        match self.tokens.next() {
            Some(TokenTree::Ident(ident)) => Ok(ident),
            Some(other) => Err(Error::new(other.span(), "expected identifier")),
            None => Err(Error::new(self.span, "expected identifier, found end of input")),
        }
    }

    pub fn parse_punct(&mut self, expected: char) -> Result<Punct> {
        match self.tokens.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == expected => Ok(p),
            Some(other) => Err(Error::new(other.span(), format!("expected `{expected}`"))),
            None => Err(Error::new(self.span, format!("expected `{expected}`, found end of input"))),
        }
    }

    pub fn parse_literal(&mut self) -> Result<Literal> {
        match self.tokens.next() {
            Some(TokenTree::Literal(lit)) => Ok(lit),
            Some(other) => Err(Error::new(other.span(), "expected literal")),
            None => Err(Error::new(self.span, "expected literal, found end of input")),
        }
    }

    pub fn parse_group(&mut self, delimiter: Delimiter) -> Result<Group> {
        match self.tokens.next() {
            Some(TokenTree::Group(g)) if g.delimiter() == delimiter => Ok(g),
            Some(other) => {
                let expected = match delimiter {
                    Delimiter::Parenthesis => "parentheses",
                    Delimiter::Brace => "braces",
                    Delimiter::Bracket => "brackets",
                    Delimiter::None => "group",
                };
                Err(Error::new(other.span(), format!("expected {expected}")))
            }
            None => {
                let expected = match delimiter {
                    Delimiter::Parenthesis => "parentheses",
                    Delimiter::Brace => "braces",
                    Delimiter::Bracket => "brackets",
                    Delimiter::None => "group",
                };
                Err(Error::new(self.span, format!("expected {expected}, found end of input")))
            }
        }
    }

    /// Parse a comma-separated list of items, calling the provided function for each.
    pub fn parse_terminated<T, F>(&mut self, mut parse_fn: F) -> Result<Vec<T>>
    where
        F: FnMut(&mut ParseStream<'a>) -> Result<T>,
    {
        let mut items = Vec::new();
        while !self.is_empty() {
            items.push(parse_fn(self)?);
            if self.peek_punct() == Some(',') {
                self.parse_punct(',')?;
            } else {
                break;
            }
        }
        Ok(items)
    }

    /// Verify the stream is empty, returning an error if not.
    pub fn expect_empty(&mut self) -> Result<()> {
        if let Some(token) = self.next_token() {
            Err(Error::new(token.span(), "expected `,`"))
        } else {
            Ok(())
        }
    }
}

/// Parse an integer literal.
pub fn parse_int(lit: &Literal) -> Result<usize> {
    let s = lit.to_string();
    s.parse::<usize>()
        .map_err(|_| Error::new(lit.span(), "expected integer literal"))
}

/// Parsed derive input - the enum being derived on.
pub struct DeriveInput {
    pub ident: Ident,
    pub attrs: Vec<Attribute>,
    pub data: Data,
}

pub enum Data {
    Enum(DataEnum),
    Other(Span),
}

pub struct DataEnum {
    pub variants: Vec<Variant>,
}

#[derive(Clone)]
pub struct Variant {
    pub ident: Ident,
    pub attrs: Vec<Attribute>,
    pub fields: Fields,
}

impl std::hash::Hash for Variant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ident.to_string().hash(state);
    }
}

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.ident.to_string() == other.ident.to_string()
    }
}

impl Eq for Variant {}

#[derive(Clone)]
pub enum Fields {
    Unit,
    Named(Vec<Field>),
    Unnamed(Vec<Field>),
}

#[derive(Clone)]
pub struct Field {
    pub ident: Option<Ident>,
    pub attrs: Vec<Attribute>,
    pub span: Span,
}

#[derive(Clone)]
pub struct Attribute {
    pub path: Vec<Ident>,
    pub tokens: TokenStream,
    pub span: Span,
}

impl Attribute {
    pub fn path_is_ident(&self, ident: &str) -> bool {
        self.path.len() == 1 && self.path[0].to_string() == ident
    }

    /// Check if this is a path-only attribute (no arguments).
    pub fn is_path(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Check if this is a list attribute like `#[attr(args)]`.
    pub fn is_list(&self) -> bool {
        let mut iter = self.tokens.clone().into_iter();
        matches!(iter.next(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
    }

    /// Check if this is a name-value attribute like `#[attr = value]`.
    pub fn is_name_value(&self) -> bool {
        let mut iter = self.tokens.clone().into_iter();
        matches!(iter.next(), Some(TokenTree::Punct(p)) if p.as_char() == '=')
    }

    /// Get the arguments of a list attribute.
    pub fn parse_args(&self) -> Result<TokenStream> {
        let mut iter = self.tokens.clone().into_iter();
        match iter.next() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => Ok(g.stream()),
            _ => Err(Error::new(self.span, "expected parenthesized arguments")),
        }
    }
}

/// Parse a `proc_macro::TokenStream` into a `DeriveInput`.
pub fn parse_derive_input(input: proc_macro::TokenStream) -> Result<DeriveInput> {
    let tokens: TokenStream = input.into();
    let mut stream = ParseStream::new(tokens);

    // Parse outer attributes
    let mut attrs = Vec::new();
    while stream.peek_punct() == Some('#') {
        attrs.push(parse_outer_attribute(&mut stream)?);
    }

    // Parse visibility (skip if present)
    if stream.peek_ident().is_some_and(|i| i.to_string() == "pub") {
        stream.parse_ident()?; // pub
        // Handle `pub(crate)`, `pub(super)`, etc.
        if stream.peek_group(Delimiter::Parenthesis).is_some() {
            stream.parse_group(Delimiter::Parenthesis)?;
        }
    }

    // Parse `enum` or `struct` keyword
    let keyword = stream.parse_ident()?;
    let keyword_str = keyword.to_string();

    // Parse name
    let ident = stream.parse_ident()?;

    // Skip generics if present (< ... >)
    if stream.peek_punct() == Some('<') {
        skip_generics(&mut stream)?;
    }

    // Skip where clause if present
    if stream.peek_ident().is_some_and(|i| i.to_string() == "where") {
        skip_where_clause(&mut stream)?;
    }

    // Parse body
    let data = if keyword_str == "enum" {
        let body = stream.parse_group(Delimiter::Brace)?;
        let variants = parse_enum_variants(body)?;
        Data::Enum(DataEnum { variants })
    } else {
        // Use call_site span so error points to derive macro
        Data::Other(Span::call_site())
    };

    Ok(DeriveInput { ident, attrs, data })
}

fn parse_outer_attribute(stream: &mut ParseStream) -> Result<Attribute> {
    let start_span = stream.span();
    stream.parse_punct('#')?;
    let group = stream.parse_group(Delimiter::Bracket)?;
    let mut inner = ParseStream::new(group.stream());

    // Parse attribute path (e.g., `derive`, `enum_from`, `doc`)
    let mut path = vec![inner.parse_ident()?];
    while inner.peek_punct() == Some(':') {
        inner.parse_punct(':')?;
        inner.parse_punct(':')?;
        path.push(inner.parse_ident()?);
    }

    // Remaining tokens are the attribute arguments
    let tokens: TokenStream = inner.tokens.collect();

    Ok(Attribute {
        path,
        tokens,
        span: start_span,
    })
}

fn skip_generics(stream: &mut ParseStream) -> Result<()> {
    stream.parse_punct('<')?;
    let mut depth = 1;
    while depth > 0 {
        match stream.next_token() {
            Some(TokenTree::Punct(p)) if p.as_char() == '<' => depth += 1,
            Some(TokenTree::Punct(p)) if p.as_char() == '>' => depth -= 1,
            Some(_) => {}
            None => return Err(Error::new(stream.span(), "unexpected end of generics")),
        }
    }
    Ok(())
}

fn skip_where_clause(stream: &mut ParseStream) -> Result<()> {
    stream.parse_ident()?; // where
    // Skip until we hit a `{` or `;`
    loop {
        match stream.tokens.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break,
            Some(TokenTree::Punct(p)) if p.as_char() == ';' => break,
            Some(_) => {
                stream.next_token();
            }
            None => break,
        }
    }
    Ok(())
}

fn parse_enum_variants(body: Group) -> Result<Vec<Variant>> {
    let mut stream = ParseStream::new(body.stream());
    let mut variants = Vec::new();

    while !stream.is_empty() {
        // Parse attributes
        let mut attrs = Vec::new();
        while stream.peek_punct() == Some('#') {
            attrs.push(parse_outer_attribute(&mut stream)?);
        }

        // Parse variant name
        let ident = stream.parse_ident()?;

        // Parse fields
        let fields = if stream.peek_group(Delimiter::Parenthesis).is_some() {
            let group = stream.parse_group(Delimiter::Parenthesis)?;
            let fields = parse_tuple_fields(group)?;
            Fields::Unnamed(fields)
        } else if stream.peek_group(Delimiter::Brace).is_some() {
            let group = stream.parse_group(Delimiter::Brace)?;
            let fields = parse_struct_fields(group)?;
            Fields::Named(fields)
        } else {
            Fields::Unit
        };

        // Skip discriminant if present (= expr)
        if stream.peek_punct() == Some('=') {
            stream.parse_punct('=')?;
            // Skip until comma or end
            while stream.peek_punct() != Some(',') && !stream.is_empty() {
                stream.next_token();
            }
        }

        variants.push(Variant { ident, attrs, fields });

        // Parse comma if present
        if stream.peek_punct() == Some(',') {
            stream.parse_punct(',')?;
        }
    }

    Ok(variants)
}

fn parse_tuple_fields(group: Group) -> Result<Vec<Field>> {
    let mut stream = ParseStream::new(group.stream());
    let mut fields = Vec::new();

    while !stream.is_empty() {
        let span = stream.span();

        // Parse attributes
        let mut attrs = Vec::new();
        while stream.peek_punct() == Some('#') {
            attrs.push(parse_outer_attribute(&mut stream)?);
        }

        // Skip type tokens until comma or end
        skip_type(&mut stream)?;

        fields.push(Field {
            ident: None,
            attrs,
            span,
        });

        if stream.peek_punct() == Some(',') {
            stream.parse_punct(',')?;
        }
    }

    Ok(fields)
}

fn parse_struct_fields(group: Group) -> Result<Vec<Field>> {
    let mut stream = ParseStream::new(group.stream());
    let mut fields = Vec::new();

    while !stream.is_empty() {
        let span = stream.span();

        // Parse attributes
        let mut attrs = Vec::new();
        while stream.peek_punct() == Some('#') {
            attrs.push(parse_outer_attribute(&mut stream)?);
        }

        // Skip visibility if present
        if stream.peek_ident().is_some_and(|i| i.to_string() == "pub") {
            stream.parse_ident()?;
            if stream.peek_group(Delimiter::Parenthesis).is_some() {
                stream.parse_group(Delimiter::Parenthesis)?;
            }
        }

        // Parse field name
        let ident = stream.parse_ident()?;

        // Parse colon
        stream.parse_punct(':')?;

        // Skip type
        skip_type(&mut stream)?;

        fields.push(Field {
            ident: Some(ident),
            attrs,
            span,
        });

        if stream.peek_punct() == Some(',') {
            stream.parse_punct(',')?;
        }
    }

    Ok(fields)
}

fn skip_type(stream: &mut ParseStream) -> Result<()> {
    let mut depth = 0;
    loop {
        match stream.tokens.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == ',' && depth == 0 => break,
            Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
                depth += 1;
                stream.next_token();
            }
            Some(TokenTree::Punct(p)) if p.as_char() == '>' => {
                depth -= 1;
                stream.next_token();
            }
            Some(TokenTree::Group(_)) => {
                stream.next_token();
            }
            Some(_) => {
                stream.next_token();
            }
            None => break,
        }
    }
    Ok(())
}
