//! defines `Expr` (Expression, the minimum executing unit of Erg).
use std::borrow::Borrow;
use std::fmt;

use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::ty::SubrKind;
use erg_common::value::ValueObj;
use erg_common::Str;
use erg_common::{
    fmt_option, fmt_vec, impl_display_for_enum, impl_display_for_single_struct,
    impl_display_from_nested, impl_displayable_stream_for_wrapper, impl_locational,
    impl_locational_for_enum, impl_nested_display_for_enum, impl_stream, impl_stream_for_wrapper,
};

use crate::token::{Token, TokenKind};

pub fn fmt_lines<'a, T: NestedDisplay + 'a>(
    mut iter: impl Iterator<Item = &'a T>,
    f: &mut fmt::Formatter<'_>,
    level: usize,
) -> fmt::Result {
    if let Some(line) = iter.next() {
        line.fmt_nest(f, level)?;
    }
    for arg in iter {
        write!(f, "\n")?;
        arg.fmt_nest(f, level)?;
    }
    Ok(())
}

/// リテラルに実際の値が格納された構造体(定数畳み込み用)
/// ArrayやDictはまた別に
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Literal {
    pub token: Token,
}

impl NestedDisplay for Literal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.token)
    }
}

impl_display_from_nested!(Literal);

impl Locational for Literal {
    #[inline]
    fn loc(&self) -> Location {
        self.token.loc()
    }
}

impl From<Token> for Literal {
    #[inline]
    fn from(token: Token) -> Self {
        Self { token }
    }
}

impl From<&Literal> for ValueObj {
    #[inline]
    fn from(lit: &Literal) -> ValueObj {
        ValueObj::from(&lit.token)
    }
}

impl Literal {
    #[inline]
    pub fn is(&self, kind: TokenKind) -> bool {
        self.token.is(kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PosArg {
    pub expr: Expr,
}

impl NestedDisplay for PosArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.expr.fmt_nest(f, level)
    }
}

impl_display_from_nested!(PosArg);

impl Locational for PosArg {
    fn loc(&self) -> Location {
        self.expr.loc()
    }
}

impl PosArg {
    pub const fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KwArg {
    pub keyword: Token,
    pub expr: Expr,
}

impl NestedDisplay for KwArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "{}:\n", self.keyword)?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(KwArg);

impl Locational for KwArg {
    fn loc(&self) -> Location {
        Location::concat(&self.keyword, &self.expr)
    }
}

impl KwArg {
    pub const fn new(keyword: Token, expr: Expr) -> Self {
        Self { keyword, expr }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Args {
    pos_args: Vec<PosArg>,
    kw_args: Vec<KwArg>,
    paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        fmt_lines(self.pos_args.iter(), f, level)
    }
}

impl_display_from_nested!(Args);

impl Locational for Args {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.paren {
            Location::concat(l, r)
        } else {
            Location::concat(&self.pos_args[0], self.pos_args.last().unwrap())
        }
    }
}

// impl_stream!(Args, Arg, args);

impl Args {
    pub const fn new(
        pos_args: Vec<PosArg>,
        kw_args: Vec<KwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            kw_args,
            paren,
        }
    }

    pub const fn empty() -> Self {
        Self::new(vec![], vec![], None)
    }

    // for replacing to hir::Args
    pub fn deconstruct(self) -> (Vec<PosArg>, Vec<KwArg>, Option<(Token, Token)>) {
        (self.pos_args, self.kw_args, self.paren)
    }

    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
    }

    pub fn kw_is_empty(&self) -> bool {
        self.kw_args.is_empty()
    }

    pub fn pos_args(&self) -> &[PosArg] {
        &self.pos_args[..]
    }

    pub fn kw_args(&self) -> &[KwArg] {
        &self.kw_args[..]
    }

    pub fn into_iters(
        self,
    ) -> (
        impl IntoIterator<Item = PosArg>,
        impl IntoIterator<Item = KwArg>,
    ) {
        (self.pos_args.into_iter(), self.kw_args.into_iter())
    }

    pub fn push_pos(&mut self, arg: PosArg) {
        self.pos_args.push(arg);
    }

    pub fn remove_pos(&mut self, index: usize) -> PosArg {
        self.pos_args.remove(index)
    }

    pub fn insert_pos(&mut self, index: usize, arg: PosArg) {
        self.pos_args.insert(index, arg);
    }

    pub fn push_kw(&mut self, arg: KwArg) {
        self.kw_args.push(arg);
    }
}

/// represents a local variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Local {
    pub symbol: Token,
}

impl_display_for_single_struct!(Local, symbol.content);

impl Locational for Local {
    #[inline]
    fn loc(&self) -> Location {
        self.symbol.loc()
    }
}

impl Local {
    pub const fn new(symbol: Token) -> Self {
        Self { symbol }
    }

    pub fn dummy(name: &'static str) -> Self {
        Self::new(Token::from_str(TokenKind::Symbol, name))
    }

    // &strにするとクローンしたいときにアロケーションコストがかかるので&Strのままで
    pub const fn inspect(&self) -> &Str {
        &self.symbol.content
    }

    pub fn is_const(&self) -> bool {
        self.symbol.inspect().chars().next().unwrap().is_uppercase()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub obj: Box<Expr>,
    pub name: Local,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}).{}", self.obj, self.name)
    }
}

impl_locational!(Attribute, obj, name);

impl Attribute {
    pub fn new(obj: Expr, name: Local) -> Self {
        Self {
            obj: Box::new(obj),
            name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TupleAttribute {
    pub obj: Box<Expr>,
    pub index: Literal,
}

impl fmt::Display for TupleAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}).{}", self.obj, self.index)
    }
}

impl_locational!(TupleAttribute, obj, index);

impl TupleAttribute {
    pub fn new(obj: Expr, index: Literal) -> Self {
        Self {
            obj: Box::new(obj),
            index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subscript {
    obj: Box<Expr>,
    index: Box<Expr>,
}

impl fmt::Display for Subscript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})[{}]", self.obj, self.index)
    }
}

impl_locational!(Subscript, obj, index);

impl Subscript {
    pub fn new(obj: Expr, index: Expr) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Accessor {
    Local(Local),
    SelfDot(Local),
    Attr(Attribute),
    TupleAttr(TupleAttribute),
    Subscr(Subscript),
}

impl NestedDisplay for Accessor {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Local(name) => write!(f, "{}", name),
            Self::SelfDot(attr) => write!(f, "self.{}", attr),
            Self::Attr(attr) => write!(f, "{}", attr),
            Self::TupleAttr(attr) => write!(f, "{}", attr),
            Self::Subscr(subscr) => write!(f, "{}", subscr),
        }
    }
}

impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Local, SelfDot, Attr, TupleAttr, Subscr);

impl Accessor {
    pub const fn local(symbol: Token) -> Self {
        Self::Local(Local::new(symbol))
    }

    pub const fn self_dot(attr: Token) -> Self {
        Self::SelfDot(Local::new(attr))
    }

    pub fn attr(obj: Expr, name: Local) -> Self {
        Self::Attr(Attribute::new(obj, name))
    }

    pub fn subscr(obj: Expr, index: Expr) -> Self {
        Self::Subscr(Subscript::new(obj, index))
    }

    pub const fn name(&self) -> Option<&Str> {
        match self {
            Self::Local(local) => Some(local.inspect()),
            Self::SelfDot(local) => Some(local.inspect()),
            _ => None,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Local(local) | Self::SelfDot(local) => local.is_const(),
            Self::Subscr(subscr) => subscr.obj.is_const_acc(),
            Self::TupleAttr(attr) => attr.obj.is_const_acc(),
            Self::Attr(attr) => attr.obj.is_const_acc() && attr.name.is_const(),
        }
    }
}

/// DictはキーつきArray(型としては別物)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Array {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elems: Args,
    pub guard: Option<Box<Expr>>,
}

impl NestedDisplay for Array {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "[{} | {}]", self.elems, guard)
        } else {
            write!(f, "[{}]", self.elems)
        }
    }
}

impl_display_from_nested!(Array);
impl_locational!(Array, l_sqbr, r_sqbr);

impl Array {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: Args, guard: Option<Expr>) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
            guard: guard.map(Box::new),
        }
    }
}

/// DictはキーつきArrayとして実現される
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dict {
    l_brace: Token,
    r_brace: Token,
    pub attrs: Args,
    guard: Option<Box<Expr>>,
}

impl NestedDisplay for Dict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "{{{} | {guard}}}", self.attrs)
        } else {
            write!(f, "{{{}}}", self.attrs)
        }
    }
}

impl_display_from_nested!(Dict);
impl_locational!(Dict, l_brace, r_brace);

impl Dict {
    pub fn new(l_brace: Token, r_brace: Token, attrs: Args, guard: Option<Expr>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BinOp {
    pub op: Token,
    pub args: [Box<Expr>; 2],
}

impl NestedDisplay for BinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "`{}`:\n", self.op.content)?;
        self.args[0].fmt_nest(f, level + 1)?;
        write!(f, "\n")?;
        self.args[1].fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(BinOp);

impl Locational for BinOp {
    fn loc(&self) -> Location {
        Location::concat(&self.op, self.args[1].as_ref())
    }
}

impl BinOp {
    pub fn new(op: Token, lhs: Expr, rhs: Expr) -> Self {
        Self {
            op,
            args: [Box::new(lhs), Box::new(rhs)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaryOp {
    pub op: Token,
    pub args: [Box<Expr>; 1],
}

impl NestedDisplay for UnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "`{}`:\n", self.op.content)?;
        self.args[0].fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(UnaryOp);

impl Locational for UnaryOp {
    fn loc(&self) -> Location {
        Location::concat(&self.op, self.args[0].as_ref())
    }
}

impl UnaryOp {
    pub fn new(op: Token, expr: Expr) -> Self {
        Self {
            op,
            args: [Box::new(expr)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Call {
    pub obj: Box<Expr>,
    pub args: Args,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "({}):\n", self.obj)?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Call);

impl Locational for Call {
    fn loc(&self) -> Location {
        if self.args.is_empty() {
            self.obj.loc()
        } else {
            Location::concat(self.obj.as_ref(), &self.args)
        }
    }
}

impl Call {
    pub fn new(obj: Expr, args: Args) -> Self {
        Self {
            obj: Box::new(obj),
            args,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block(Vec<Expr>);

impl NestedDisplay for Block {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
    }
}

impl_display_from_nested!(Block);

impl Locational for Block {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

impl_stream_for_wrapper!(Block, Expr);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstLocal {
    pub symbol: Token,
}

impl NestedDisplay for ConstLocal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl_display_from_nested!(ConstLocal);

impl Locational for ConstLocal {
    #[inline]
    fn loc(&self) -> Location {
        self.symbol.loc()
    }
}

impl ConstLocal {
    pub const fn new(symbol: Token) -> Self {
        Self { symbol }
    }

    pub fn dummy(name: &'static str) -> Self {
        Self::new(Token::from_str(TokenKind::Symbol, name))
    }

    // &strにするとクローンしたいときにアロケーションコストがかかるので&Strのままで
    pub const fn inspect(&self) -> &Str {
        &self.symbol.content
    }
}

/// type variables
pub type ConstVar = ConstLocal;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstAttribute {
    pub obj: Box<ConstExpr>,
    pub name: ConstLocal,
}

impl NestedDisplay for ConstAttribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({}).{}", self.obj, self.name)
    }
}

impl_display_from_nested!(ConstAttribute);
impl_locational!(ConstAttribute, obj, name);

impl ConstAttribute {
    pub fn new(expr: ConstExpr, name: ConstLocal) -> Self {
        Self {
            obj: Box::new(expr),
            name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstSubscript {
    obj: Box<ConstExpr>,
    index: Box<ConstExpr>,
}

impl NestedDisplay for ConstSubscript {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({})[{}]", self.obj, self.index)
    }
}

impl_display_from_nested!(ConstSubscript);
impl_locational!(ConstSubscript, obj, index);

impl ConstSubscript {
    pub fn new(obj: ConstExpr, index: ConstExpr) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstAccessor {
    Local(ConstLocal),
    SelfDot(ConstLocal),
    Attr(ConstAttribute),
    Subscr(ConstSubscript),
}

impl_nested_display_for_enum!(ConstAccessor; Local, SelfDot, Attr, Subscr);
impl_display_from_nested!(ConstAccessor);
impl_locational_for_enum!(ConstAccessor; Local, SelfDot, Attr, Subscr);

impl ConstAccessor {
    pub const fn local(symbol: Token) -> Self {
        Self::Local(ConstLocal::new(symbol))
    }

    pub const fn dot_self(attr: Token) -> Self {
        Self::SelfDot(ConstLocal::new(attr))
    }

    pub fn attr(obj: ConstExpr, name: ConstLocal) -> Self {
        Self::Attr(ConstAttribute::new(obj, name))
    }

    pub fn subscr(obj: ConstExpr, index: ConstExpr) -> Self {
        Self::Subscr(ConstSubscript::new(obj, index))
    }
}

/// DictはキーつきArray(型としては別物)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArray {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elems: ConstArgs,
    pub guard: Option<Box<ConstExpr>>,
}

impl NestedDisplay for ConstArray {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "[{} | {}]", self.elems, guard)
        } else {
            write!(f, "[{}]", self.elems)
        }
    }
}

impl_display_from_nested!(ConstArray);
impl_locational!(ConstArray, l_sqbr, r_sqbr);

impl ConstArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: ConstArgs, guard: Option<ConstExpr>) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDict {
    l_brace: Token,
    r_brace: Token,
    pub attrs: ConstArgs,
}

impl NestedDisplay for ConstDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.attrs)
    }
}

impl_display_from_nested!(ConstDict);
impl_locational!(ConstDict, l_brace, r_brace);

impl ConstDict {
    pub fn new(l_brace: Token, r_brace: Token, attrs: ConstArgs) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstBinOp {
    pub op: Token,
    pub lhs: Box<ConstExpr>,
    pub rhs: Box<ConstExpr>,
}

impl NestedDisplay for ConstBinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "`{}`({}, {})", self.op.content, self.lhs, self.rhs)
    }
}

impl_display_from_nested!(ConstBinOp);
impl_locational!(ConstBinOp, lhs, rhs);

impl ConstBinOp {
    pub fn new(op: Token, lhs: ConstExpr, rhs: ConstExpr) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstUnaryOp {
    pub op: Token,
    pub expr: Box<ConstExpr>,
}

impl NestedDisplay for ConstUnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "`{}`({})", self.op.content, self.expr)
    }
}

impl_display_from_nested!(ConstUnaryOp);
impl_locational!(ConstUnaryOp, op, expr);

impl ConstUnaryOp {
    pub fn new(op: Token, expr: ConstExpr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

/// Application
/// ex. `Vec Int` of `Option Vec Int`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstApp {
    pub acc: ConstAccessor,
    pub args: ConstArgs,
}

impl NestedDisplay for ConstApp {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "({})", self.acc)?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl Locational for ConstApp {
    fn loc(&self) -> Location {
        if self.args.is_empty() {
            self.acc.loc()
        } else {
            Location::concat(&self.acc, &self.args)
        }
    }
}

impl ConstApp {
    pub const fn new(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self { acc, args }
    }
}

/// valid expression for an argument of polymorphic types
/// 多相型の実引数として有効な式
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpr {
    Lit(Literal),
    Erased(Literal), // _
    Accessor(ConstAccessor),
    App(ConstApp),
    Array(ConstArray),
    // Dict(Dict),
    // Set(Set),
    Dict(ConstDict),
    BinOp(ConstBinOp),
    UnaryOp(ConstUnaryOp),
}

impl_nested_display_for_enum!(ConstExpr; Lit, Accessor, App, Array, Dict, BinOp, UnaryOp, Erased);
impl_display_from_nested!(ConstExpr);
impl_locational_for_enum!(ConstExpr; Lit, Accessor, App, Array, Dict, BinOp, UnaryOp, Erased);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstPosArg {
    pub expr: ConstExpr,
}

impl NestedDisplay for ConstPosArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.expr.fmt_nest(f, level)
    }
}

impl Locational for ConstPosArg {
    fn loc(&self) -> Location {
        self.expr.loc()
    }
}

impl ConstPosArg {
    pub const fn new(expr: ConstExpr) -> Self {
        Self { expr }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstKwArg {
    pub keyword: Token,
    pub expr: ConstExpr,
}

impl NestedDisplay for ConstKwArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "{}: ", self.keyword.inspect())?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl Locational for ConstKwArg {
    fn loc(&self) -> Location {
        Location::concat(&self.keyword, &self.expr)
    }
}

impl ConstKwArg {
    pub const fn new(keyword: Token, expr: ConstExpr) -> Self {
        Self { keyword, expr }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArgs {
    pos_args: Vec<ConstPosArg>,
    kw_args: Vec<ConstKwArg>,
    paren: Option<(Token, Token)>,
}

impl NestedDisplay for ConstArgs {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        for arg in self.pos_args.iter() {
            arg.fmt_nest(f, level)?;
            write!(f, ", ")?;
        }
        write!(f, "?")?;
        for arg in self.kw_args.iter() {
            arg.fmt_nest(f, level)?;
            write!(f, ", ")?;
        }
        Ok(())
    }
}

impl_display_from_nested!(ConstArgs);

impl Locational for ConstArgs {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.paren {
            Location::concat(l, r)
        } else if let Some(last) = self.kw_args.last() {
            Location::concat(self.pos_args.first().unwrap(), last)
        } else if let Some(last) = self.pos_args.last() {
            Location::concat(self.pos_args.first().unwrap(), last)
        } else {
            unreachable!()
        }
    }
}

// impl_stream!(ConstArgs, ConstKwArg, pos_args);

impl ConstArgs {
    pub const fn new(
        pos_args: Vec<ConstPosArg>,
        kw_args: Vec<ConstKwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            kw_args,
            paren,
        }
    }

    pub const fn empty() -> Self {
        Self::new(vec![], vec![], None)
    }

    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
    }

    pub fn pos_args(&self) -> impl Iterator<Item = &ConstPosArg> {
        self.pos_args.iter()
    }

    pub fn kw_args(&self) -> impl Iterator<Item = &ConstKwArg> {
        self.kw_args.iter()
    }

    pub fn into_iters(
        self,
    ) -> (
        impl IntoIterator<Item = ConstPosArg>,
        impl IntoIterator<Item = ConstKwArg>,
    ) {
        (self.pos_args.into_iter(), self.kw_args.into_iter())
    }

    pub fn push_pos(&mut self, arg: ConstPosArg) {
        self.pos_args.push(arg);
    }

    pub fn push_kw(&mut self, arg: ConstKwArg) {
        self.kw_args.push(arg);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SimpleTypeSpec {
    pub name: VarName,
    pub args: ConstArgs, // args can be nested (e.g. Vec Vec Int)
}

impl fmt::Display for SimpleTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}{}", self.name, self.args)
        }
    }
}

impl Locational for SimpleTypeSpec {
    fn loc(&self) -> Location {
        if let Some(last) = self.args.kw_args.last() {
            Location::concat(&self.name, last)
        } else if let Some(last) = self.args.pos_args.last() {
            Location::concat(&self.name, last)
        } else {
            self.name.loc()
        }
    }
}

impl SimpleTypeSpec {
    pub const fn new(name: VarName, args: ConstArgs) -> Self {
        Self { name, args }
    }
}

// OK:
//   ts = [T, U]; x: ts[0] = ...
//   ts = {.T: T, .U: U}; x: ts.T = ...
//   ...; x: foo.bar.ts[0] = ...
// NG:
//   ts = {"T": T, "U": U}; x: ts["T"] = ...
//   f T = T; x: f(T) = ...
//   ...; x: foo[0].T = ...
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PreDeclTypeSpec {
    Simple(SimpleTypeSpec),
    Attr {
        namespace: Vec<VarName>,
        t: SimpleTypeSpec,
    },
    Subscr {
        namespace: Vec<VarName>,
        name: VarName,
        index: Token,
    },
}

impl fmt::Display for PreDeclTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreDeclTypeSpec::Simple(ts) => write!(f, "{}", ts),
            PreDeclTypeSpec::Attr { namespace, t } => {
                write!(f, "{}.{}", namespace.join("."), t)
            }
            PreDeclTypeSpec::Subscr {
                namespace,
                name,
                index,
            } => {
                write!(f, "{}.{}[{}]", namespace.join("."), name, index)
            }
        }
    }
}

impl Locational for PreDeclTypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Simple(s) => s.loc(),
            Self::Attr { namespace, t } => Location::concat(&namespace[0], t),
            Self::Subscr {
                namespace, index, ..
            } => Location::concat(&namespace[0], index),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamTySpec {
    pub name: Option<Token>,
    pub ty: TypeSpec,
}

impl fmt::Display for ParamTySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{}: {}", name.inspect(), self.ty)
        } else {
            write!(f, "{}", self.ty)
        }
    }
}

impl Locational for ParamTySpec {
    fn loc(&self) -> Location {
        if let Some(name) = &self.name {
            Location::concat(name, &self.ty)
        } else {
            self.ty.loc()
        }
    }
}

impl ParamTySpec {
    pub const fn new(name: Option<Token>, ty: TypeSpec) -> Self {
        Self { name, ty }
    }

    pub const fn anonymous(ty: TypeSpec) -> Self {
        Self::new(None, ty)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrTySpec {
    pub kind: SubrKind,
    pub lparen: Option<Token>,
    pub non_defaults: Vec<ParamTySpec>,
    pub defaults: Vec<ParamTySpec>,
    pub return_t: Box<TypeSpec>,
}

impl fmt::Display for SubrTySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, ? {}) {} {}",
            fmt_vec(&self.non_defaults),
            fmt_vec(&self.defaults),
            self.kind.arrow(),
            self.return_t
        )
    }
}

impl Locational for SubrTySpec {
    fn loc(&self) -> Location {
        if let Some(lparen) = &self.lparen {
            Location::concat(lparen, self.return_t.as_ref())
        } else {
            // FIXME: only default subrs
            Location::concat(self.non_defaults.first().unwrap(), self.return_t.as_ref())
        }
    }
}

impl SubrTySpec {
    pub fn new(
        kind: SubrKind,
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        defaults: Vec<ParamTySpec>,
        return_t: TypeSpec,
    ) -> Self {
        Self {
            kind,
            lparen,
            non_defaults,
            defaults,
            return_t: Box::new(return_t),
        }
    }
}

/// * Array: `[Int; 3]`, `[Int, Ratio, Complex]`, etc.
/// * Dict: `[Str: Str]`, etc.
/// * Option: `Int?`, etc.
/// * And (Intersection type): Add and Sub and Mul (== Num), etc.
/// * Not (Diff type): Pos == Nat not {0}, etc.
/// * Or (Union type): Int or None (== Option Int), etc.
/// * Enum: `{0, 1}` (== Binary), etc.
/// * Range: 1..12, 0.0<..1.0, etc.
/// * Record: {.into_s: Self.() -> Str }, etc.
/// * Func: Int -> Int, etc.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeSpec {
    PreDeclTy(PreDeclTypeSpec),
    /* Composite types */
    Array {
        t: PreDeclTypeSpec,
        len: ConstExpr,
    },
    Tuple(Vec<TypeSpec>),
    // Dict(),
    // Option(),
    And(Box<TypeSpec>, Box<TypeSpec>),
    Not(Box<TypeSpec>, Box<TypeSpec>),
    Or(Box<TypeSpec>, Box<TypeSpec>),
    Enum(ConstArgs),
    Interval {
        op: Token,
        lhs: ConstExpr,
        rhs: ConstExpr,
    },
    // Record(),
    Subr(SubrTySpec),
}

impl fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreDeclTy(ty) => write!(f, "{ty}"),
            Self::And(lhs, rhs) => write!(f, "{lhs} and {rhs}"),
            Self::Not(lhs, rhs) => write!(f, "{lhs} not {rhs}"),
            Self::Or(lhs, rhs) => write!(f, "{lhs} or {rhs}"),
            Self::Array { t, len } => write!(f, "[{t}; {len}]"),
            Self::Tuple(tys) => write!(f, "({})", fmt_vec(&tys)),
            Self::Enum(elems) => write!(f, "{{{elems}}}"),
            Self::Interval { op, lhs, rhs } => write!(f, "{lhs}{}{rhs}", op.inspect()),
            Self::Subr(s) => write!(f, "{s}"),
        }
    }
}

impl Locational for TypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::PreDeclTy(sig) => sig.loc(),
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                Location::concat(lhs.as_ref(), rhs.as_ref())
            }
            Self::Array { t, len } => Location::concat(t, len),
            // TODO: ユニット
            Self::Tuple(tys) => Location::concat(tys.first().unwrap(), tys.last().unwrap()),
            Self::Enum(set) => set.loc(),
            Self::Interval { lhs, rhs, .. } => Location::concat(lhs, rhs),
            Self::Subr(s) => s.loc(),
        }
    }
}

impl TypeSpec {
    pub fn and(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    pub fn not(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::Not(Box::new(lhs), Box::new(rhs))
    }

    pub fn or(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    pub const fn interval(op: Token, lhs: ConstExpr, rhs: ConstExpr) -> Self {
        Self::Interval { op, lhs, rhs }
    }

    pub fn func(
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        defaults: Vec<ParamTySpec>,
        return_t: TypeSpec,
    ) -> Self {
        Self::Subr(SubrTySpec::new(
            SubrKind::Func,
            lparen,
            non_defaults,
            defaults,
            return_t,
        ))
    }

    pub fn proc(
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        defaults: Vec<ParamTySpec>,
        return_t: TypeSpec,
    ) -> Self {
        Self::Subr(SubrTySpec::new(
            SubrKind::Proc,
            lparen,
            non_defaults,
            defaults,
            return_t,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeBoundSpec {
    Subtype { sub: VarName, sup: TypeSpec }, // e.g. S <: Show
    Instance { name: VarName, ty: TypeSpec }, // e.g. N: Nat
                                             // Predicate, // TODO: e.g. N > 5
}

impl NestedDisplay for TypeBoundSpec {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Subtype { sub, sup } => write!(f, "{} <: {}", sub, sup),
            Self::Instance { name, ty } => write!(f, "{} : {}", name, ty),
        }
    }
}

impl_display_from_nested!(TypeBoundSpec);

impl Locational for TypeBoundSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Subtype { sub: l, sup: r } | Self::Instance { name: l, ty: r } => {
                Location::concat(l, r)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeBoundSpecs(Vec<TypeBoundSpec>);

impl_displayable_stream_for_wrapper!(TypeBoundSpecs, TypeBoundSpec);

impl Locational for TypeBoundSpecs {
    fn loc(&self) -> Location {
        Location::concat(self.first().unwrap(), self.last().unwrap())
    }
}

/// デコレータは関数を返す関数オブジェクトならば何でも指定できる
/// e.g. @(x -> x)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decorator(Expr);

impl Decorator {
    pub const fn new(expr: Expr) -> Self {
        Self(expr)
    }
}

/// symbol as a left value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarName(Token);

impl Borrow<str> for VarName {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0.content[..]
    }
}

impl Borrow<Str> for VarName {
    #[inline]
    fn borrow(&self) -> &Str {
        &self.0.content
    }
}

impl Locational for VarName {
    #[inline]
    fn loc(&self) -> Location {
        self.0.loc()
    }
}

impl fmt::Display for VarName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.inspect())
    }
}

impl VarName {
    pub const fn new(symbol: Token) -> Self {
        Self(symbol)
    }

    pub const fn from_static(symbol: &'static str) -> Self {
        Self(Token::static_symbol(symbol))
    }

    pub fn from_str(symbol: Str) -> Self {
        Self(Token::from_str(TokenKind::Symbol, &symbol))
    }

    #[inline]
    pub fn is_const(&self) -> bool {
        self.0
            .content
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_procedural(&self) -> bool {
        self.0
            .content
            .chars()
            .last()
            .map(|c| c == '!')
            .unwrap_or(false)
    }

    pub const fn token(&self) -> &Token {
        &self.0
    }

    pub fn into_token(self) -> Token {
        self.0
    }

    pub const fn inspect(&self) -> &Str {
        &self.0.content
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarArrayPattern {
    l_sqbr: Token,
    pub(crate) elems: Vars,
    r_sqbr: Token,
}

impl fmt::Display for VarArrayPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.elems)
    }
}

impl_locational!(VarArrayPattern, l_sqbr, r_sqbr);

impl Stream<VarSignature> for VarArrayPattern {
    #[inline]
    fn payload(self) -> Vec<VarSignature> {
        self.elems.payload()
    }
    #[inline]
    fn ref_payload(&self) -> &Vec<VarSignature> {
        self.elems.ref_payload()
    }
    #[inline]
    fn ref_mut_payload(&mut self) -> &mut Vec<VarSignature> {
        self.elems.ref_mut_payload()
    }
}

impl VarArrayPattern {
    pub const fn new(l_sqbr: Token, elems: Vars, r_sqbr: Token) -> Self {
        Self {
            l_sqbr,
            elems,
            r_sqbr,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarTuplePattern {
    paren: Option<(Token, Token)>,
    pub(crate) elems: Vars,
}

impl fmt::Display for VarTuplePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.elems)
    }
}

impl Locational for VarTuplePattern {
    fn loc(&self) -> Location {
        match &self.paren {
            Some((l, r)) => Location::concat(l, r),
            None => Location::concat(&self.elems[0], self.elems.last().unwrap()),
        }
    }
}

impl Stream<VarSignature> for VarTuplePattern {
    #[inline]
    fn payload(self) -> Vec<VarSignature> {
        self.elems.payload()
    }
    #[inline]
    fn ref_payload(&self) -> &Vec<VarSignature> {
        self.elems.ref_payload()
    }
    #[inline]
    fn ref_mut_payload(&mut self) -> &mut Vec<VarSignature> {
        self.elems.ref_mut_payload()
    }
}

impl VarTuplePattern {
    pub const fn new(paren: Option<(Token, Token)>, elems: Vars) -> Self {
        Self { paren, elems }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarRecordPattern {
    l_brace: Token,
    // TODO: レコード専用の構造体を作る
    pub(crate) elems: Vars,
    r_brace: Token,
}

impl fmt::Display for VarRecordPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", self.elems)
    }
}

impl_locational!(VarRecordPattern, l_brace, r_brace);

impl VarRecordPattern {
    pub const fn new(l_brace: Token, elems: Vars, r_brace: Token) -> Self {
        Self {
            l_brace,
            elems,
            r_brace,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VarPattern {
    Discard(Token),
    VarName(VarName),
    SelfDot(VarName), // only self-attribute can assign once
    /// e.g. `[x, y, z]` of `[x, y, z] = [1, 2, 3]`
    Array(VarArrayPattern),
    /// e.g. `(x, y, z)` of `(x, y, z) = (1, 2, 3)`
    Tuple(VarTuplePattern),
    // e.g. `{name; age}`, `{_; [car, cdr]}`
    Record(VarRecordPattern),
}

impl NestedDisplay for VarPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Discard(_) => write!(f, "_"),
            Self::VarName(n) => write!(f, "{}", n),
            Self::SelfDot(n) => write!(f, "self.{}", n),
            Self::Array(a) => write!(f, "{}", a),
            Self::Tuple(t) => write!(f, "{}", t),
            Self::Record(r) => write!(f, "{}", r),
        }
    }
}

impl_display_from_nested!(VarPattern);
impl_locational_for_enum!(VarPattern; Discard, VarName, SelfDot, Array, Tuple, Record);

impl VarPattern {
    pub const fn inspect(&self) -> Option<&Str> {
        match self {
            Self::VarName(n) | Self::SelfDot(n) => Some(n.inspect()),
            _ => None,
        }
    }

    pub fn inspects(&self) -> Vec<&Str> {
        match self {
            Self::VarName(n) | Self::SelfDot(n) => vec![n.inspect()],
            Self::Array(VarArrayPattern { elems, .. })
            | Self::Tuple(VarTuplePattern { elems, .. })
            | Self::Record(VarRecordPattern { elems, .. }) => {
                elems.iter().map(|s| s.pat.inspects()).flatten().collect()
            }
            _ => vec![],
        }
    }

    // _!(...) = ... is invalid
    pub fn is_procedural(&self) -> bool {
        match self {
            Self::VarName(n) | Self::SelfDot(n) => n.is_procedural(),
            _ => false,
        }
    }

    // _ = (type block) is invalid
    pub fn is_const(&self) -> bool {
        match self {
            Self::VarName(n) | Self::SelfDot(n) => n.is_const(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarSignature {
    pub pat: VarPattern,
    pub t_spec: Option<TypeSpec>,
}

impl NestedDisplay for VarSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}{}", self.pat, fmt_option!(pre ": ", &self.t_spec))
    }
}

impl_display_from_nested!(VarSignature);

impl Locational for VarSignature {
    fn loc(&self) -> Location {
        if let Some(t_spec) = &self.t_spec {
            Location::concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
    }
}

impl VarSignature {
    pub const fn new(pat: VarPattern, t_spec: Option<TypeSpec>) -> Self {
        Self { pat, t_spec }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub fn is_const(&self) -> bool {
        self.pat.is_const()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vars {
    elems: Vec<VarSignature>,
}

impl NestedDisplay for Vars {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec(&self.elems))
    }
}

impl_display_from_nested!(Vars);
impl_stream!(Vars, VarSignature, elems);

impl Vars {
    pub const fn new(elems: Vec<VarSignature>) -> Self {
        Self { elems }
    }

    pub const fn empty() -> Self {
        Self::new(vec![])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamArrayPattern {
    l_sqbr: Token,
    pub elems: Params,
    r_sqbr: Token,
}

impl NestedDisplay for ParamArrayPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}]", self.elems)
    }
}

impl_display_from_nested!(ParamArrayPattern);
impl_locational!(ParamArrayPattern, l_sqbr, r_sqbr);

impl ParamArrayPattern {
    pub const fn new(l_sqbr: Token, elems: Params, r_sqbr: Token) -> Self {
        Self {
            l_sqbr,
            elems,
            r_sqbr,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }
    pub fn len(&self) -> usize {
        self.elems.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamRecordPattern {
    l_brace: Token,
    pub(crate) elems: Params,
    r_brace: Token,
}

impl NestedDisplay for ParamRecordPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.elems)
    }
}

impl_display_from_nested!(ParamRecordPattern);
impl_locational!(ParamRecordPattern, l_brace, r_brace);

impl ParamRecordPattern {
    pub const fn new(l_brace: Token, elems: Params, r_brace: Token) -> Self {
        Self {
            l_brace,
            elems,
            r_brace,
        }
    }
}

/// 関数定義や無名関数で使えるパターン
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamPattern {
    Discard(Token),
    VarName(VarName),
    // TODO: ConstField(),
    // e.g. `a` of `[*a, b] = [1, 2, 3]` (a == [1, 2], b == 3)
    //      `b` of `[a, *b] = [1, 2, 3]` (a == 1, b == [2, 3])
    VarArgsName(VarName),
    Lit(Literal),
    Array(ParamArrayPattern),
    Record(ParamRecordPattern),
}

impl_display_for_enum!(ParamPattern; Discard, VarName, VarArgsName, Lit, Array, Record);
impl_locational_for_enum!(ParamPattern; Discard, VarName, VarArgsName, Lit, Array, Record);

impl ParamPattern {
    pub const fn inspect(&self) -> Option<&Str> {
        match self {
            Self::VarName(n) | Self::VarArgsName(n) => Some(n.inspect()),
            _ => None,
        }
    }

    pub const fn is_lit(&self) -> bool {
        matches!(self, Self::Lit(_))
    }

    pub fn is_procedural(&self) -> bool {
        match self {
            Self::Discard(_) => true,
            Self::VarName(n) | Self::VarArgsName(n) => n.is_procedural(),
            _ => false,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Discard(_) => true,
            Self::VarName(n) | Self::VarArgsName(n) => n.is_const(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NonDefaultParamSignature {
    pub pat: ParamPattern,
    pub t_spec: Option<TypeSpec>,
}

impl NestedDisplay for NonDefaultParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{}{}", self.pat, fmt_option!(pre ": ", &self.t_spec))
    }
}

impl_display_from_nested!(NonDefaultParamSignature);

impl Locational for NonDefaultParamSignature {
    fn loc(&self) -> Location {
        if let Some(t_spec) = &self.t_spec {
            Location::concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
    }
}

impl NonDefaultParamSignature {
    pub const fn new(pat: ParamPattern, t_spec: Option<TypeSpec>) -> Self {
        Self { pat, t_spec }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefaultParamSignature {
    pub pat: ParamPattern,
    pub t_spec: Option<TypeSpec>,
    pub val: ConstExpr,
}

impl NestedDisplay for DefaultParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(
            f,
            "{}{} |= {}",
            self.pat,
            fmt_option!(pre ": ", &self.t_spec),
            self.val
        )
    }
}

impl_display_from_nested!(DefaultParamSignature);

impl Locational for DefaultParamSignature {
    fn loc(&self) -> Location {
        Location::concat(&self.pat, &self.val)
    }
}

impl DefaultParamSignature {
    pub const fn new(pat: ParamPattern, t_spec: Option<TypeSpec>, val: ConstExpr) -> Self {
        Self { pat, t_spec, val }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }
}

pub trait ParamSig {
    fn pat(&self) -> &ParamPattern;
    fn t_spec(&self) -> Option<&TypeSpec>;
}

impl ParamSig for NonDefaultParamSignature {
    fn pat(&self) -> &ParamPattern {
        &self.pat
    }
    fn t_spec(&self) -> Option<&TypeSpec> {
        self.t_spec.as_ref()
    }
}

impl ParamSig for DefaultParamSignature {
    fn pat(&self) -> &ParamPattern {
        &self.pat
    }
    fn t_spec(&self) -> Option<&TypeSpec> {
        self.t_spec.as_ref()
    }
}

impl ParamSig for &NonDefaultParamSignature {
    fn pat(&self) -> &ParamPattern {
        &self.pat
    }
    fn t_spec(&self) -> Option<&TypeSpec> {
        self.t_spec.as_ref()
    }
}

impl ParamSig for &DefaultParamSignature {
    fn pat(&self) -> &ParamPattern {
        &self.pat
    }
    fn t_spec(&self) -> Option<&TypeSpec> {
        self.t_spec.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    pub non_defaults: Vec<NonDefaultParamSignature>,
    pub defaults: Vec<DefaultParamSignature>,
    pub parens: Option<(Token, Token)>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {})",
            fmt_vec(&self.non_defaults),
            fmt_vec(&self.defaults)
        )
    }
}

impl Locational for Params {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.parens {
            Location::concat(l, r)
        } else if !self.non_defaults.is_empty() {
            Location::concat(&self.non_defaults[0], self.non_defaults.last().unwrap())
        } else if !self.defaults.is_empty() {
            Location::concat(&self.defaults[0], self.defaults.last().unwrap())
        } else {
            panic!()
        }
    }
}

impl Params {
    pub const fn new(
        non_defaults: Vec<NonDefaultParamSignature>,
        defaults: Vec<DefaultParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            defaults,
            parens,
        }
    }

    pub fn deconstruct(
        self,
    ) -> (
        Vec<NonDefaultParamSignature>,
        Vec<DefaultParamSignature>,
        Option<(Token, Token)>,
    ) {
        (self.non_defaults, self.defaults, self.parens)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.non_defaults.len() + self.defaults.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 引数を取るならTypeでもSubr扱い
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrSignature {
    pub decorators: Set<Decorator>,
    pub name: VarName,
    pub params: Params,
    pub return_t_spec: Option<TypeSpec>,
    pub bounds: TypeBoundSpecs,
}

impl NestedDisplay for SubrSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.bounds.is_empty() {
            write!(
                f,
                "{}{}{}",
                self.name,
                self.params,
                fmt_option!(pre ": ", &self.return_t_spec)
            )
        } else {
            write!(
                f,
                "{}|{}|{}{}",
                self.name,
                self.bounds,
                self.params,
                fmt_option!(pre ": ", &self.return_t_spec)
            )
        }
    }
}

impl_display_from_nested!(SubrSignature);

impl Locational for SubrSignature {
    fn loc(&self) -> Location {
        if !self.bounds.is_empty() {
            Location::concat(&self.name, &self.bounds)
        } else if let Some(return_t) = &self.return_t_spec {
            Location::concat(&self.name, return_t)
        } else {
            Location::concat(&self.name, &self.params)
        }
    }
}

impl SubrSignature {
    pub const fn new(
        decorators: Set<Decorator>,
        name: VarName,
        params: Params,
        return_t: Option<TypeSpec>,
        bounds: TypeBoundSpecs,
    ) -> Self {
        Self {
            decorators,
            name,
            params,
            return_t_spec: return_t,
            bounds,
        }
    }

    pub fn is_const(&self) -> bool {
        self.name.is_const()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LambdaSignature {
    pub params: Params,
    pub return_t_spec: Option<TypeSpec>,
    pub bounds: TypeBoundSpecs,
}

impl fmt::Display for LambdaSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bounds.is_empty() {
            write!(
                f,
                "{}{}",
                self.params,
                fmt_option!(pre ": ", &self.return_t_spec)
            )
        } else {
            write!(
                f,
                "|{}|{}{}",
                self.bounds,
                self.params,
                fmt_option!(pre ": ", &self.return_t_spec)
            )
        }
    }
}

impl Locational for LambdaSignature {
    fn loc(&self) -> Location {
        if !self.bounds.is_empty() {
            Location::concat(&self.params, &self.bounds)
        } else if let Some(return_t) = &self.return_t_spec {
            Location::concat(&self.params, return_t)
        } else if self.params.is_empty() && self.params.parens.is_none() {
            unreachable!()
        } else {
            self.params.loc()
        }
    }
}

impl LambdaSignature {
    pub const fn new(params: Params, return_t: Option<TypeSpec>, bounds: TypeBoundSpecs) -> Self {
        Self {
            params,
            return_t_spec: return_t,
            bounds,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

impl DefId {
    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub sig: LambdaSignature,
    /// for detecting func/proc
    pub op: Token,
    pub body: Block,
    pub id: DefId,
}

impl NestedDisplay for Lambda {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{} {}\n", self.sig, self.op.content)?;
        self.body.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Lambda);

impl Lambda {
    pub const fn new(sig: LambdaSignature, op: Token, body: Block, id: DefId) -> Self {
        Self { sig, op, body, id }
    }

    pub fn is_procedural(&self) -> bool {
        self.op.is(TokenKind::ProcArrow)
    }
}

impl_locational!(Lambda, sig, body);

/// represents a declaration of a variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Signature {
    Var(VarSignature),
    Subr(SubrSignature),
}

impl_nested_display_for_enum!(Signature; Var, Subr);
impl_display_from_nested!(Signature);
impl_locational_for_enum!(Signature; Var, Subr);

impl Signature {
    pub fn name_as_str(&self) -> &Str {
        match self {
            Self::Var(v) => v.pat.inspect().unwrap(),
            Self::Subr(s) => &s.name.inspect(),
        }
    }

    pub fn name(&self) -> Option<&VarName> {
        match self {
            Self::Var(v) => {
                if let VarPattern::VarName(v) = &v.pat {
                    Some(v)
                } else {
                    None
                }
            }
            Self::Subr(s) => Some(&s.name),
        }
    }

    pub fn t_spec(&self) -> Option<&TypeSpec> {
        match self {
            Self::Var(v) => v.t_spec.as_ref(),
            Self::Subr(c) => c.return_t_spec.as_ref(),
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Var(v) => v.is_const(),
            Self::Subr(s) => s.is_const(),
        }
    }
}

pub type Decl = Signature;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefBody {
    pub op: Token,
    pub block: Block,
    pub id: DefId,
}

impl_locational!(DefBody, op, block);

impl DefBody {
    pub const fn new(op: Token, block: Block, id: DefId) -> Self {
        Self { op, block, id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Def {
    pub sig: Signature,
    pub body: DefBody,
}

impl NestedDisplay for Def {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{} {}\n", self.sig, self.body.op.content)?;
        self.body.block.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Def);
impl_locational!(Def, sig, body);

impl Def {
    pub const fn new(sig: Signature, body: DefBody) -> Self {
        Self { sig, body }
    }

    pub fn is_const(&self) -> bool {
        self.sig.is_const()
    }

    pub const fn is_subr(&self) -> bool {
        matches!(&self.sig, Signature::Subr(_))
    }
}

/// Expression(式)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Lit(Literal),
    Accessor(Accessor),
    Array(Array),
    // Dict(Dict),
    // Set(Set),
    Dict(Dict),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Call(Call),
    Lambda(Lambda),
    Decl(Decl),
    Def(Def),
}

impl_nested_display_for_enum!(Expr; Lit, Accessor, Array, Dict, BinOp, UnaryOp, Call, Lambda, Decl, Def);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Lit, Accessor, Array, Dict, BinOp, UnaryOp, Call, Lambda, Decl, Def);

impl Expr {
    pub fn is_match_call(&self) -> bool {
        matches!(self, Expr::Call(Call{ obj, .. }) if obj.get_name().map(|s| &s[..] == "match").unwrap_or(false))
    }

    pub fn is_const_acc(&self) -> bool {
        matches!(self, Expr::Accessor(acc) if acc.is_const())
    }

    pub fn get_name(&self) -> Option<&Str> {
        match self {
            Expr::Accessor(acc) => acc.name(),
            _ => None,
        }
    }

    pub fn local(name: &str, lineno: usize, col_begin: usize) -> Self {
        Self::Accessor(Accessor::local(Token::new(
            TokenKind::Symbol,
            Str::rc(name),
            lineno,
            col_begin,
        )))
    }

    pub fn dummy_local(name: &str) -> Self {
        Self::Accessor(Accessor::local(Token::from_str(TokenKind::Symbol, name)))
    }

    pub fn static_local(name: &'static str) -> Self {
        Self::Accessor(Accessor::local(Token::static_symbol(name)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module(Vec<Expr>);

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_lines(self.0.iter(), f, 0)
    }
}

impl Locational for Module {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

impl_stream_for_wrapper!(Module, Expr);

#[derive(Debug)]
pub struct AST {
    pub name: Str,
    pub module: Module,
}

impl_display_for_single_struct!(AST, module);

impl AST {
    pub const fn new(name: Str, module: Module) -> Self {
        Self { name, module }
    }

    pub fn is_empty(&self) -> bool {
        self.module.is_empty()
    }
}
