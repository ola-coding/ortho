//! The parser, written by hand: one method per rule of the grammar, named after it,
//! setting the properties the rule sets. A keyword decides nearly every choice; the
//! few it cannot decide try one alternative and fall back on the other.
use super::SyntaxError;
use super::ast::{Ast, NodeId, Prop as P, Type as T, Value};
use super::lexer::{Kind, Token, tokenize};

/// A rule did not match. What was expected is on the parser.
struct Fail;
type R<T> = Result<T, Fail>;

struct Parser<'a> {
    ast: &'a mut Ast,
    tokens: Vec<Token<'a>>,
    pos: usize,
    /// The token no attempt got past, which is where the syntax error is, and what
    /// would have gone on from there.
    farthest: usize,
    expected: Vec<&'static str>,
}

pub fn parse(ast: &mut Ast, text: &str) -> Result<NodeId, SyntaxError> {
    let tokens = tokenize(text)?;
    let len = ast.len();
    let mut parser = Parser { ast, tokens, pos: 0, farthest: 0, expected: Vec::new() };
    parser.model().map_err(|Fail| {
        let error = parser.syntax_error();
        parser.ast.truncate(len);
        error
    })
}

const VISIBILITIES: &[&str] = &["private", "protected", "public"];
#[rustfmt::skip]
const DEFINITION_KINDS: &[&str] = &[
    "item", "connection", "flow", "allocation", "occurrence", "calc", "constraint", "state", "requirement", "concern",
    "case", "analysis", "verification", "view", "viewpoint", "rendering", "metadata", "enum",
];
#[rustfmt::skip]
const USAGE_KINDS: &[&str] = &[
    "item", "occurrence", "enum", "calc", "constraint", "state", "requirement", "concern", "case", "analysis",
    "verification", "viewpoint", "rendering",
];
/// The operators by precedence, tightest first; `^` and `**` associate to the right.
const OPERATORS: &[&[&str]] = &[
    &["^", "**"],
    &["*", "/", "%"],
    &["+", "-"],
    &[".."],
    &["<", ">", "<=", ">="],
    &["istype", "hastype", "@", "@@", "as", "meta"],
    &["==", "!=", "===", "!=="],
    &["&"],
    &["xor"],
    &["|"],
    &["and"],
    &["or"],
    &["implies"],
    &["??"],
];

fn precedence(operator: &str) -> usize {
    OPERATORS.iter().position(|level| level.contains(&operator)).unwrap()
}

/// A quoted name is the same name without its quotes, its escapes resolved (§8.2.2.1.2).
fn unquote(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name[1..name.len() - 1].chars();
    while let Some(c) = chars.next() {
        out.push(if c == '\\' { unescape(chars.next().unwrap()) } else { c });
    }
    out
}

/// A string's text: as `unquote`, and `\0` too.
fn string_value(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\\' => match chars.next().unwrap() {
                '0' => '\0',
                escaped => unescape(escaped),
            },
            c => c,
        });
    }
    out
}

fn unescape(c: char) -> char {
    match c {
        'b' => '\u{8}',
        'f' => '\u{c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\u{b}',
        c => c,
    }
}

type ReadText<'a> = fn(&mut Parser<'a>) -> R<String>;
type ReadNode<'a> = fn(&mut Parser<'a>) -> R<NodeId>;

impl<'a> Parser<'a> {
    // ------------------------------------------------------------ Tokens

    fn tok(&self, n: usize) -> Token<'a> {
        self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn at_n(&self, n: usize, word: &str) -> bool {
        let t = self.tok(n);
        t.kind == Kind::Word && t.text == word
    }

    fn at(&self, word: &str) -> bool {
        self.at_n(0, word)
    }

    fn at_any(&self, words: &[&str]) -> bool {
        let t = self.tok(0);
        t.kind == Kind::Word && words.contains(&t.text)
    }

    fn name_at(&self, n: usize) -> bool {
        matches!(self.tok(n).kind, Kind::Id | Kind::Unrestricted)
    }

    fn start(&self) -> u32 {
        self.tok(0).start as u32
    }

    /// Notes that `what` would have been fine here, if this is as far as any attempt got.
    fn note(&mut self, what: &'static str) {
        if self.pos > self.farthest {
            self.farthest = self.pos;
            self.expected.clear();
        }
        if self.pos == self.farthest && !self.expected.contains(&what) {
            self.expected.push(what);
        }
    }

    fn fail<T>(&mut self, what: &'static str) -> R<T> {
        self.note(what);
        Err(Fail)
    }

    fn syntax_error(&self) -> SyntaxError {
        let quoted: Vec<String> = self
            .expected
            .iter()
            .map(|what| if what.contains(' ') { what.to_string() } else { format!("'{what}'") })
            .collect();
        let expected = match quoted.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
            None => "something else".to_string(),
        };
        let token = self.tokens[self.farthest];
        let found = if token.kind == Kind::Eof {
            "the end of the file".to_string()
        } else if token.kind == Kind::Comment {
            "a comment".to_string()
        } else {
            format!("'{}'", token.text)
        };
        SyntaxError { offset: token.start, message: format!("Expected {expected} but found {found}.") }
    }

    fn eat(&mut self, word: &str) -> bool {
        let found = self.at(word);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect(&mut self, word: &'static str) -> R<()> {
        if self.eat(word) { Ok(()) } else { self.fail(word) }
    }

    /// Runs `rule`, whose assignments go on `n`; if it fails, as if it had never run.
    fn attempt(&mut self, n: NodeId, rule: impl FnOnce(&mut Self) -> R<()>) -> bool {
        let (pos, assigned, nodes) = (self.pos, self.ast[n].fields.len(), self.ast.len());
        if rule(self).is_ok() {
            return true;
        }
        self.pos = pos;
        self.ast[n].fields.truncate(assigned);
        self.ast.truncate(nodes);
        false
    }

    // ------------------------------------------------------------ Assigning

    fn node(&mut self, ty: T) -> NodeId {
        let offset = self.start();
        self.ast.add(ty, offset)
    }

    fn set(&mut self, n: NodeId, prop: P, list: bool, offset: u32, value: Value) {
        self.ast[n].fields.push(super::ast::Field { prop, list, offset, value });
    }

    /// `prop?='word'`
    fn flag_if(&mut self, word: &str, n: NodeId, prop: P) -> bool {
        let offset = self.start();
        let found = self.eat(word);
        if found {
            self.set(n, prop, false, offset, Value::Flag);
        }
        found
    }

    /// `prop=` the keyword here, already looked at.
    fn keyword(&mut self, n: NodeId, prop: P) {
        let t = self.tok(0);
        self.pos += 1;
        self.set(n, prop, false, t.start as u32, Value::Text(t.text.to_string()));
    }

    /// `prop=` what `read` reads, kept as text.
    fn text(&mut self, n: NodeId, prop: P, read: ReadText<'a>) -> R<()> {
        let offset = self.start();
        let text = read(self)?;
        self.set(n, prop, false, offset, Value::Text(text));
        Ok(())
    }

    /// `prop+=` what `read` reads, kept as text.
    fn add_text(&mut self, n: NodeId, prop: P, read: ReadText<'a>) -> R<()> {
        let offset = self.start();
        let text = read(self)?;
        self.set(n, prop, true, offset, Value::Text(text));
        Ok(())
    }

    /// `prop=[...]`, a reference.
    fn reference(&mut self, n: NodeId, prop: P) -> R<()> {
        let offset = self.start();
        let text = self.qualified_name()?;
        self.set(n, prop, false, offset, Value::Reference { text, target: None });
        Ok(())
    }

    /// `prop+=[...]`, a reference named by `read`.
    fn add_reference(&mut self, n: NodeId, prop: P, read: ReadText<'a>) -> R<()> {
        let offset = self.start();
        let text = read(self)?;
        self.set(n, prop, true, offset, Value::Reference { text, target: None });
        Ok(())
    }

    fn attach(&mut self, n: NodeId, prop: P, list: bool, child: NodeId) {
        self.ast[child].parent = Some(n);
        let offset = self.ast[child].offset;
        self.set(n, prop, list, offset, Value::Child(child));
    }

    /// `prop=` what `read` reads, a node.
    fn child(&mut self, n: NodeId, prop: P, read: ReadNode<'a>) -> R<()> {
        let child = read(self)?;
        self.attach(n, prop, false, child);
        Ok(())
    }

    /// `prop+=` what `read` reads, a node.
    fn add(&mut self, n: NodeId, prop: P, read: ReadNode<'a>) -> R<()> {
        let child = read(self)?;
        self.attach(n, prop, true, child);
        Ok(())
    }

    /// `a, b, c`, each added as `add_one` adds it.
    fn separated(&mut self, n: NodeId, prop: P, add_one: fn(&mut Self, NodeId, P) -> R<()>) -> R<()> {
        loop {
            add_one(self, n, prop)?;
            if !self.eat(",") {
                return Ok(());
            }
        }
    }

    fn add_name(&mut self, n: NodeId, prop: P) -> R<()> {
        self.add_text(n, prop, Self::qualified_name)
    }

    fn add_qualified_reference(&mut self, n: NodeId, prop: P) -> R<()> {
        self.add_reference(n, prop, Self::qualified_name)
    }

    fn add_chain(&mut self, n: NodeId, prop: P) -> R<()> {
        self.add_text(n, prop, Self::feature_chain)
    }

    fn add_type_name(&mut self, n: NodeId, prop: P) -> R<()> {
        self.add_text(n, prop, Self::type_name)
    }

    fn add_expression(&mut self, n: NodeId, prop: P) -> R<()> {
        self.add(n, prop, Self::expression)
    }

    // ------------------------------------------------------------ Model and packages

    fn model(&mut self) -> R<NodeId> {
        let model = self.node(T::Model);
        if self.tok(0).kind == Kind::Eof {
            self.ast[model].offset = 0;
        }
        while self.tok(0).kind != Kind::Eof {
            let visible = usize::from(self.at_any(VISIBILITIES));
            if self.package_ahead() {
                self.add(model, P::Packages, Self::package_decl)?;
            } else if visible == 1 && self.at_n(1, "import") {
                self.add(model, P::Imports, Self::import_decl)?;
            } else if self.at_n(visible, "alias") {
                self.add(model, P::Aliases, Self::alias_member)?;
            } else if self.annotation_ahead() {
                self.add(model, P::Annotations, Self::annotating_element)?;
            } else {
                self.add(model, P::Members, Self::element)?;
            }
        }
        Ok(model)
    }

    /// Whether a package's prefix and its keyword are next.
    fn package_ahead(&self) -> bool {
        let mut i = usize::from(self.at_any(VISIBILITIES));
        if self.at_n(i, "standard") {
            i += 1;
        }
        if self.at_n(i, "library") {
            i += 1;
        }
        while self.at_n(i, "#") && self.name_at(i + 1) {
            i += 2;
            while self.at_n(i, "::") && self.name_at(i + 1) {
                i += 2;
            }
        }
        self.at_n(i, "package")
    }

    fn package_decl(&mut self) -> R<NodeId> {
        let n = self.node(T::PackageDecl);
        self.visibility(n);
        if self.flag_if("standard", n, P::IsStandard) && !self.at("library") {
            return self.fail("library");
        }
        self.flag_if("library", n, P::IsLibrary);
        self.prefix_metadata(n)?;
        self.expect("package")?;
        self.identification(n)?;
        self.body(n)?;
        Ok(n)
    }

    fn visibility(&mut self, n: NodeId) {
        if self.at_any(VISIBILITIES) {
            self.keyword(n, P::Visibility);
        }
    }

    fn import_decl(&mut self) -> R<NodeId> {
        let n = self.node(T::ImportDecl);
        self.visibility(n);
        self.expect("import")?;
        self.flag_if("all", n, P::All);
        self.reference(n, P::Target)?;
        self.namespace_ending(n)?;
        Ok(n)
    }

    /// `::*`, `::**`, filters and a body, which an import and an expose share.
    fn namespace_ending(&mut self, n: NodeId) -> R<()> {
        self.flag_if("::*", n, P::Wildcard);
        self.flag_if("::**", n, P::Recursive);
        while self.eat("[") {
            self.add_expression(n, P::Filters)?;
            self.expect("]")?;
        }
        self.relationship_body(n)
    }

    fn alias_member(&mut self) -> R<NodeId> {
        let n = self.node(T::AliasMember);
        self.visibility(n);
        self.expect("alias")?;
        if self.eat("<") {
            self.text(n, P::ShortName, Self::name)?;
            self.expect(">")?;
        }
        if self.name_at(0) {
            self.text(n, P::Name, Self::name)?;
        }
        self.expect("for")?;
        self.text(n, P::Target, Self::qualified_name)?;
        self.relationship_body(n)?;
        Ok(n)
    }

    fn relationship_body(&mut self, n: NodeId) -> R<()> {
        if self.eat(";") {
            return Ok(());
        }
        self.note(";");
        self.expect("{")?;
        while !self.eat("}") {
            if !self.annotation_ahead() {
                return self.fail("}");
            }
            self.add(n, P::Annotations, Self::annotating_element)?;
        }
        Ok(())
    }

    fn body(&mut self, n: NodeId) -> R<()> {
        if self.eat(";") {
            return Ok(());
        }
        self.note(";");
        self.expect("{")?;
        loop {
            if self.eat("}") {
                return Ok(());
            }
            let visible = usize::from(self.at_any(VISIBILITIES));
            if visible == 1 && self.at_n(1, "import") {
                self.add(n, P::Imports, Self::import_decl)?;
            } else if self.at_n(visible, "alias") {
                self.add(n, P::Aliases, Self::alias_member)?;
            } else if self.annotation_ahead() {
                self.add(n, P::Annotations, Self::annotating_element)?;
            } else if self.at_n(visible, "expose") {
                self.add(n, P::Exposes, Self::expose)?;
            } else if self.at_n(visible, "filter") {
                self.add(n, P::Filters, Self::element_filter)?;
            } else if !self.member_or_result(n)? {
                return Ok(());
            }
        }
    }

    /// A member, else the expression that closes the body. False once closed.
    fn member_or_result(&mut self, n: NodeId) -> R<bool> {
        let (pos, nodes) = (self.pos, self.ast.len());
        if self.add(n, P::Members, Self::member).is_ok() {
            return Ok(true);
        }
        self.pos = pos;
        self.ast.truncate(nodes);
        self.child(n, P::Result, Self::expression)?;
        self.expect("}")?;
        Ok(false)
    }

    fn member(&mut self) -> R<NodeId> {
        if self.package_ahead() { self.package_decl() } else { self.element() }
    }

    fn element_filter(&mut self) -> R<NodeId> {
        let n = self.node(T::ElementFilter);
        self.visibility(n);
        self.expect("filter")?;
        self.child(n, P::Condition, Self::expression)?;
        self.expect(";")?;
        Ok(n)
    }

    fn expose(&mut self) -> R<NodeId> {
        let n = self.node(T::Expose);
        self.visibility(n);
        self.expect("expose")?;
        self.reference(n, P::Element)?;
        self.namespace_ending(n)?;
        Ok(n)
    }

    // ------------------------------------------------------------ Comments

    fn annotation_ahead(&self) -> bool {
        self.tok(0).kind == Kind::Comment || self.at_any(&["comment", "locale", "doc", "rep", "language"])
    }

    fn annotating_element(&mut self) -> R<NodeId> {
        if self.at("doc") {
            let n = self.node(T::Documentation);
            self.pos += 1;
            self.identification(n)?;
            self.locale(n)?;
            self.comment_body(n)?;
            Ok(n)
        } else if self.at("rep") || self.at("language") {
            let n = self.node(T::TextualRepresentation);
            if self.eat("rep") {
                self.identification(n)?;
            }
            self.expect("language")?;
            self.text(n, P::Language, Self::string)?;
            self.comment_body(n)?;
            Ok(n)
        } else {
            let n = self.node(T::Comment);
            if self.eat("comment") {
                self.identification(n)?;
                if self.eat("about") {
                    self.separated(n, P::About, Self::add_name)?;
                }
            }
            self.locale(n)?;
            self.comment_body(n)?;
            Ok(n)
        }
    }

    fn locale(&mut self, n: NodeId) -> R<()> {
        if self.eat("locale") {
            self.text(n, P::Locale, Self::string)?;
        }
        Ok(())
    }

    fn comment_body(&mut self, n: NodeId) -> R<()> {
        let t = self.tok(0);
        if t.kind != Kind::Comment {
            return self.fail("a comment");
        }
        self.pos += 1;
        self.set(n, P::Body, false, t.start as u32, Value::Text(t.text.to_string()));
        Ok(())
    }

    fn string(&mut self) -> R<String> {
        if self.tok(0).kind != Kind::Str {
            return self.fail("a string");
        }
        let value = string_value(self.tok(0).text);
        self.pos += 1;
        Ok(value)
    }

    // ------------------------------------------------------------ Definitions and usages

    fn element(&mut self) -> R<NodeId> {
        let n = self.node(T::Element);
        self.visibility(n);
        self.flag_if("variant", n, P::IsVariant);
        self.flag_if("then", n, P::Then);
        self.flag_if("return", n, P::IsReturn);
        // `end a [1] part b`: a multiplicity here is the end's, never the usage's.
        if self.flag_if("end", n, P::IsEnd) && (self.at("[") || (self.name_at(0) && self.at_n(1, "["))) {
            if self.name_at(0) {
                self.text(n, P::CrossName, Self::name)?;
            }
            self.child(n, P::EndMultiplicity, Self::multiplicity)?;
            while self.eat("ordered") || self.eat("nonunique") {}
        }
        if self.at_any(&["in", "out", "inout"]) {
            self.keyword(n, P::Direction);
        }
        self.flag_if("derived", n, P::IsDerived);
        if !self.flag_if("abstract", n, P::IsAbstract) {
            self.flag_if("variation", n, P::IsVariation);
        }
        self.flag_if("constant", n, P::IsConstant);
        self.flag_if("ref", n, P::IsReference);
        self.flag_if("individual", n, P::IsIndividual);
        if self.at_any(&["snapshot", "timeslice"]) {
            self.keyword(n, P::PortionKind);
        }
        self.prefix_metadata(n)?;
        self.alternatives(n)?;
        Ok(n)
    }

    fn prefix_metadata(&mut self, n: NodeId) -> R<()> {
        while self.at("#") {
            let offset = self.start();
            self.pos += 1;
            let name = self.qualified_name()?;
            self.set(n, P::PrefixMetadata, true, offset, Value::Text(format!("#{name}")));
        }
        Ok(())
    }

    fn alternatives(&mut self, n: NodeId) -> R<()> {
        let t = self.tok(0);
        if t.kind != Kind::Word {
            return self.default_usage(n);
        }
        let def_next = self.at_n(1, "def");
        match t.text {
            "part" if def_next => self.def_with_references(n, T::PartDef),
            "port" if def_next => self.def_with_references(n, T::PortDef),
            "interface" if def_next => self.def_as_written(n, T::InterfaceDef, 2),
            "attribute" if def_next => self.def_as_written(n, T::AttributeDef, 2),
            "action" if def_next => self.def_as_written(n, T::ActionDef, 2),
            "use" if self.at_n(1, "case") && self.at_n(2, "def") => self.def_as_written(n, T::UseCaseDef, 3),
            "def" => self.other_definition(n),
            kind if def_next && DEFINITION_KINDS.contains(&kind) => {
                self.keyword(n, P::Kind);
                self.other_definition(n)
            }
            "part" => self.usage(n, T::PartUsage, 1, P::Type, true),
            "attribute" => self.usage(n, T::AttributeUsage, 1, P::Type, false),
            "port" => self.port_usage(n),
            "action" => self.action(n),
            "use" if self.at_n(1, "case") => self.usage(n, T::UseCaseUsage, 2, P::Type, true),
            "actor" => self.usage(n, T::ActorUsage, 1, P::Type, true),
            "subject" => self.usage(n, T::SubjectUsage, 1, P::Type, true),
            "include" => self.include_usage(n),
            "perform" => self.perform_usage(n),
            "connection" | "interface" | "connect" => self.connection_usage(n),
            "allocation" | "allocate" => self.allocation_usage(n),
            "message" => self.message_usage(n),
            "view" => self.usage(n, T::ViewUsage, 1, P::Definition, false),
            "assert" | "not" | "satisfy" => self.assertion(n),
            "binding" | "bind" => self.binding_usage(n),
            "flow" => self.flow_usage(n),
            "succession" if self.at_n(1, "flow") => self.flow_usage(n),
            "succession" | "first" => self.first_succession(n),
            "@" | "metadata" => self.metadata_usage(n),
            "dependency" => self.dependency(n),
            kind if USAGE_KINDS.contains(&kind) => {
                self.keyword(n, P::Kind);
                self.other_usage_rest(n)
            }
            "event" => self.other_usage(n, "occurrence"),
            "exhibit" => self.other_usage(n, "state"),
            "render" => self.other_usage(n, "rendering"),
            "frame" => self.other_usage(n, "concern"),
            "verify" => self.other_usage(n, "requirement"),
            "require" | "assume" => {
                self.keyword(n, P::Kind);
                self.prefix_metadata(n)?;
                self.eat("constraint");
                self.other_usage_rest(n)
            }
            "objective" | "stakeholder" | "merge" | "decide" | "join" | "fork" => {
                self.keyword(n, P::Kind);
                self.other_usage_rest(n)
            }
            "if" => self.if_element(n),
            "else" => {
                self.ast[n].ty = T::SuccessionUsage;
                self.flag_if("else", n, P::IsElse);
                self.child(n, P::Next, Self::succession_end)?;
                self.body(n)
            }
            "transition" => self.transition_usage(n),
            "entry" | "do" | "exit" => self.state_action(n),
            "accept" | "send" | "assign" | "terminate" | "while" | "loop" | "for" => self.action_form(n),
            _ => self.default_usage(n),
        }
    }

    fn def_with_references(&mut self, n: NodeId, ty: T) -> R<()> {
        self.ast[n].ty = ty;
        self.pos += 2;
        self.identification(n)?;
        if self.eat(":>") || self.eat("specializes") {
            self.separated(n, P::Supertypes, Self::add_qualified_reference)?;
        }
        self.body(n)
    }

    fn def_as_written(&mut self, n: NodeId, ty: T, keywords: usize) -> R<()> {
        self.ast[n].ty = ty;
        self.pos += keywords;
        self.identification(n)?;
        self.written_supertypes(n)?;
        self.body(n)
    }

    fn written_supertypes(&mut self, n: NodeId) -> R<()> {
        if self.eat(":>") || self.eat("specializes") {
            self.separated(n, P::Supertypes, Self::add_name)?;
        }
        Ok(())
    }

    /// `(kind)? def`, the kind already read.
    fn other_definition(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::OtherDefinition;
        self.expect("def")?;
        self.identification(n)?;
        self.written_supertypes(n)?;
        self.flag_if("parallel", n, P::IsParallel);
        self.body(n)
    }

    /// `part x :> a : T = 1 { }`, and the others of that shape.
    fn usage(&mut self, n: NodeId, ty: T, keywords: usize, typed: P, type_is_reference: bool) -> R<()> {
        self.ast[n].ty = ty;
        self.pos += keywords;
        self.identification(n)?;
        self.relations_then_type(n, typed, type_is_reference)?;
        self.value_part(n)?;
        self.body(n)
    }

    /// `FeatureRelation* ((':' | 'defined' 'by') type MoreTypes FeatureRelation*)?`
    fn relations_then_type(&mut self, n: NodeId, typed: P, type_is_reference: bool) -> R<()> {
        self.relations(n)?;
        if self.typing_keyword() {
            if type_is_reference {
                self.reference(n, typed)?;
            } else {
                self.text(n, typed, Self::qualified_name)?;
            }
            self.more_types(n)?;
            self.relations(n)?;
        }
        Ok(())
    }

    fn port_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::PortUsage;
        self.pos += 1;
        self.identification(n)?;
        self.relations(n)?;
        if self.typing_keyword() {
            self.flag_if("~", n, P::Conjugated);
            self.reference(n, P::Type)?;
            self.more_types(n)?;
            self.relations(n)?;
        }
        self.value_part(n)?;
        self.body(n)
    }

    /// After `action x`, the keyword says which action it is.
    fn action(&mut self, n: NodeId) -> R<()> {
        self.pos += 1;
        self.identification(n)?;
        if self.at_any(&["accept", "send", "assign", "terminate", "if", "while", "loop", "for"]) {
            return self.action_form(n);
        }
        self.ast[n].ty = T::ActionUsage;
        self.relations_then_type(n, P::Type, true)?;
        self.value_part(n)?;
        self.body(n)
    }

    fn action_form(&mut self, n: NodeId) -> R<()> {
        match self.tok(0).text {
            "accept" => {
                self.ast[n].ty = T::AcceptAction;
                self.pos += 1;
                self.accept_part(n)?;
                if self.at_any(&["if", "do", "then"]) {
                    self.guard_effect_then(n)?;
                }
                self.body(n)
            }
            "send" => {
                self.ast[n].ty = T::SendAction;
                self.send_part(n)?;
                self.body(n)
            }
            "assign" => {
                self.ast[n].ty = T::AssignAction;
                self.assign_part(n)?;
                self.body(n)
            }
            "terminate" => {
                self.ast[n].ty = T::TerminateAction;
                self.pos += 1;
                if !self.at_body() {
                    self.child(n, P::Target, Self::expression)?;
                }
                self.body(n)
            }
            "if" => {
                self.ast[n].ty = T::IfAction;
                self.if_part(n)
            }
            "for" => {
                self.ast[n].ty = T::ForAction;
                self.pos += 1;
                self.text(n, P::Variable, Self::name)?;
                if self.at_typing() {
                    self.written_typing(n)?;
                }
                self.expect("in")?;
                self.child(n, P::Collection, Self::expression)?;
                self.child(n, P::LoopBody, Self::action_block)
            }
            _ => {
                self.ast[n].ty = T::LoopAction;
                if self.eat("while") {
                    self.child(n, P::Condition, Self::expression)?;
                } else {
                    self.expect("loop")?;
                }
                self.child(n, P::LoopBody, Self::action_block)?;
                if self.eat("until") {
                    self.child(n, P::Until, Self::expression)?;
                    self.expect(";")?;
                }
                Ok(())
            }
        }
    }

    fn include_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::IncludeUsage;
        self.pos += 1;
        if self.flag_if("use", n, P::UseCase) {
            self.expect("case")?;
            self.identification(n)?;
            self.relations_then_type(n, P::Type, true)?;
        } else {
            self.child(n, P::Target, Self::connector_end)?;
            self.typings_and_relations(n)?;
        }
        self.value_part(n)?;
        self.body(n)
    }

    fn perform_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::PerformUsage;
        self.pos += 1;
        if self.eat("action") {
            self.identification(n)?;
            self.typings_and_relations(n)?;
        } else {
            self.child(n, P::Target, Self::connector_end)?;
            self.relations(n)?;
        }
        self.value_part(n)?;
        self.body(n)
    }

    fn at_body(&self) -> bool {
        self.at(";") || self.at("{")
    }

    fn body_next(&mut self) -> R<()> {
        if self.at_body() {
            return Ok(());
        }
        self.note(";");
        self.fail("{")
    }

    fn connection_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::ConnectionUsage;
        if self.eat("connect") {
            self.connector_part(n)?;
            return self.body(n);
        }
        let interface = self.at("interface");
        self.keyword(n, P::Kind);
        let declared = self.attempt(n, |p| {
            p.identification(n)?;
            p.relations_then_type(n, P::Type, true)?;
            p.value_part(n)?;
            if p.eat("connect") {
                p.connector_part(n)?;
            }
            p.body_next()
        });
        if !declared {
            if !interface {
                return Err(Fail);
            }
            self.connector_part(n)?;
        }
        self.body(n)
    }

    fn allocation_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::AllocationUsage;
        if self.eat("allocate") {
            self.connector_part(n)?;
        } else {
            self.pos += 1;
            self.identification(n)?;
            self.typings_and_relations(n)?;
            if self.eat("allocate") {
                self.connector_part(n)?;
            }
        }
        self.body(n)
    }

    fn message_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::MessageUsage;
        self.pos += 1;
        let declared = self.attempt(n, |p| {
            p.identification(n)?;
            p.typings_and_relations(n)?;
            p.value_part(n)?;
            if p.eat("of") {
                p.payload_name(n)?;
                p.reference(n, P::Payload)?;
                p.payload_rest(n)?;
            }
            if p.eat("from") {
                p.connector_ends(n)?;
            }
            p.body_next()
        });
        if !declared {
            self.connector_ends(n)?;
        }
        self.body(n)
    }

    /// `source to target`
    fn connector_ends(&mut self, n: NodeId) -> R<()> {
        self.child(n, P::Source, Self::connector_end)?;
        self.expect("to")?;
        self.child(n, P::Target, Self::connector_end)
    }

    fn payload_name(&mut self, n: NodeId) -> R<()> {
        if self.name_at(0) && self.at_n(1, ":") {
            self.text(n, P::PayloadName, Self::name)?;
            self.pos += 1;
        }
        Ok(())
    }

    fn payload_rest(&mut self, n: NodeId) -> R<()> {
        if self.at("[") {
            self.child(n, P::PayloadMultiplicity, Self::multiplicity)?;
        }
        if self.eat("=") {
            self.child(n, P::PayloadValue, Self::expression)?;
        }
        Ok(())
    }

    /// `assert not satisfy`, or an `assert` of a constraint; `not` alone starts no element.
    fn assertion(&mut self, n: NodeId) -> R<()> {
        let assert = usize::from(self.at("assert"));
        let not = usize::from(self.at_n(assert, "not"));
        if self.at_n(assert + not, "satisfy") {
            self.ast[n].ty = T::SatisfyUsage;
            self.pos += assert;
            self.flag_if("not", n, P::Negated);
            self.pos += 1;
            if self.eat("requirement") {
                self.identification(n)?;
                self.typings_and_relations(n)?;
            } else {
                self.text(n, P::Requirement, Self::feature_chain)?;
                self.relations(n)?;
            }
            self.value_part(n)?;
            if self.eat("by") {
                self.text(n, P::Satisfier, Self::feature_chain)?;
            }
            return self.body(n);
        }
        if assert == 0 {
            return self.fail("satisfy");
        }
        self.keyword(n, P::Kind);
        self.eat("not");
        self.eat("constraint");
        self.other_usage_rest(n)
    }

    fn binding_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::BindingUsage;
        if self.eat("binding") {
            self.identification(n)?;
            self.typings_and_relations(n)?;
        }
        self.expect("bind")?;
        self.text(n, P::Left, Self::feature_chain)?;
        self.expect("=")?;
        self.text(n, P::Right, Self::feature_chain)?;
        self.body(n)
    }

    fn flow_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::FlowUsage;
        self.flag_if("succession", n, P::Succession);
        self.expect("flow")?;
        let declared = self.attempt(n, |p| {
            p.identification(n)?;
            p.typings_and_relations(n)?;
            p.value_part(n)?;
            if p.eat("of") {
                p.payload_name(n)?;
                p.text(n, P::Payload, Self::type_name)?;
                p.payload_rest(n)?;
            }
            if p.eat("from") {
                p.flow_ends(n)?;
            }
            p.body_next()
        });
        if !declared {
            self.flow_ends(n)?;
        }
        self.body(n)
    }

    fn flow_ends(&mut self, n: NodeId) -> R<()> {
        self.text(n, P::Source, Self::feature_chain)?;
        self.expect("to")?;
        self.text(n, P::Target, Self::feature_chain)
    }

    fn metadata_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::MetadataUsage;
        self.pos += 1;
        self.attempt(n, |p| {
            p.identification(n)?;
            if p.eat(":") {
                return Ok(());
            }
            p.note(":");
            p.expect("typed")?;
            p.expect("by")
        });
        self.text(n, P::Type, Self::qualified_name)?;
        if self.eat("about") {
            self.separated(n, P::About, Self::add_name)?;
        }
        self.body(n)
    }

    fn dependency(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::Dependency;
        self.pos += 1;
        self.attempt(n, |p| {
            p.identification(n)?;
            p.expect("from")
        });
        self.separated(n, P::Clients, Self::add_qualified_reference)?;
        self.expect("to")?;
        self.separated(n, P::Suppliers, Self::add_qualified_reference)?;
        self.relationship_body(n)
    }

    /// `kind='event' 'occurrence'?` and the like.
    fn other_usage(&mut self, n: NodeId, optional: &str) -> R<()> {
        self.keyword(n, P::Kind);
        self.eat(optional);
        self.other_usage_rest(n)
    }

    fn other_usage_rest(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::OtherUsage;
        if self.name_at(0) && (self.at_n(1, "::") || self.at_n(1, ".")) {
            self.text(n, P::Reference, Self::chained_name)?;
        } else {
            self.identification(n)?;
        }
        self.typings_and_relations(n)?;
        self.value_part(n)?;
        self.flag_if("parallel", n, P::IsParallel);
        self.body(n)
    }

    fn first_succession(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::SuccessionUsage;
        if self.eat("succession") {
            self.identification(n)?;
            self.typings_and_relations(n)?;
        }
        self.expect("first")?;
        self.child(n, P::First, Self::succession_end)?;
        if self.at("if") || self.at("then") {
            if self.eat("if") {
                self.child(n, P::Guard, Self::expression)?;
            }
            self.expect("then")?;
            self.child(n, P::Next, Self::succession_end)?;
        }
        self.body(n)
    }

    /// `if g then b;` is a succession, `if g { }` an action; the guard comes first in both.
    fn if_element(&mut self, n: NodeId) -> R<()> {
        self.pos += 1;
        let condition = self.expression()?;
        if self.eat("then") {
            self.ast[n].ty = T::SuccessionUsage;
            self.attach(n, P::Guard, false, condition);
            self.child(n, P::Next, Self::succession_end)?;
            return self.body(n);
        }
        self.ast[n].ty = T::IfAction;
        self.attach(n, P::Condition, false, condition);
        self.if_rest(n)
    }

    fn transition_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::TransitionUsage;
        self.pos += 1;
        self.attempt(n, |p| {
            p.identification(n)?;
            p.expect("first")
        });
        self.child(n, P::First, Self::succession_end)?;
        if self.eat("accept") {
            self.accept_part(n)?;
        }
        self.guard_effect_then(n)?;
        self.body(n)
    }

    /// `('if' guard)? ('do' effect)? 'then' next`
    fn guard_effect_then(&mut self, n: NodeId) -> R<()> {
        if self.eat("if") {
            self.child(n, P::Guard, Self::expression)?;
        }
        if self.eat("do") {
            self.child(n, P::Effect, Self::effect)?;
        }
        self.expect("then")?;
        self.child(n, P::Next, Self::succession_end)
    }

    fn state_action(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::StateAction;
        self.keyword(n, P::Kind);
        if !self.eat(";") {
            self.child(n, P::Action, Self::element)?;
        }
        Ok(())
    }

    fn default_usage(&mut self, n: NodeId) -> R<()> {
        self.ast[n].ty = T::DefaultUsage;
        if self.at("<") || self.name_at(0) {
            if self.eat("<") {
                self.text(n, P::ShortName, Self::name)?;
                self.expect(">")?;
            }
            self.text(n, P::Name, Self::name)?;
            self.typings_and_relations(n)?;
            self.value_part(n)?;
        } else if self.at_typing() || self.at_relation() {
            self.typings_and_relations(n)?;
            self.value_part(n)?;
        } else if self.at_value() {
            self.value_part(n)?;
        } else if self.ast.flag(n, P::IsEnd)
            && self.at_any(&[";", "{"])
            && self.ast.child(n, P::EndMultiplicity).is_some()
        {
            // `end a [1];`: with nothing after them, the name and multiplicity are the end's own.
            for field in &mut self.ast[n].fields {
                field.prop = match field.prop {
                    P::CrossName => P::Name,
                    P::EndMultiplicity => P::Multiplicity,
                    other => other,
                };
            }
        } else {
            return self.fail("a declaration");
        }
        self.body(n)
    }

    fn succession_end(&mut self) -> R<NodeId> {
        let n = self.node(T::SuccessionEnd);
        if self.at("[") {
            self.child(n, P::Multiplicity, Self::multiplicity)?;
        }
        self.text(n, P::Chain, Self::feature_chain)?;
        Ok(n)
    }

    fn accept_part(&mut self, n: NodeId) -> R<()> {
        if self.name_at(0) {
            self.text(n, P::PayloadName, Self::name)?;
        }
        self.typings_and_relations(n)?;
        if self.at_any(&["at", "after", "when"]) {
            self.keyword(n, P::Trigger);
            self.child(n, P::TriggerValue, Self::expression)?;
        }
        if self.eat("via") {
            self.child(n, P::Via, Self::expression)?;
        }
        Ok(())
    }

    fn send_part(&mut self, n: NodeId) -> R<()> {
        self.expect("send")?;
        if !self.at_any(&["via", "to", ";", "{"]) {
            self.child(n, P::Payload, Self::expression)?;
        }
        if self.eat("via") {
            self.child(n, P::Via, Self::expression)?;
        }
        if self.eat("to") {
            self.child(n, P::To, Self::expression)?;
        }
        Ok(())
    }

    fn assign_part(&mut self, n: NodeId) -> R<()> {
        self.expect("assign")?;
        self.child(n, P::Target, Self::expression)?;
        self.expect(":=")?;
        self.child(n, P::Assigned, Self::expression)
    }

    fn if_part(&mut self, n: NodeId) -> R<()> {
        self.expect("if")?;
        self.child(n, P::Condition, Self::expression)?;
        self.if_rest(n)
    }

    /// An `else` that goes with this `if`: what follows it is a block or another
    /// `if`, not the end of a succession of its own.
    fn if_rest(&mut self, n: NodeId) -> R<()> {
        self.child(n, P::ThenBlock, Self::action_block)?;
        if self.at("else") && (self.at_n(1, "{") || self.at_n(1, ";") || self.at_n(1, "action") || self.at_n(1, "if")) {
            self.pos += 1;
            if self.at("if") {
                let else_if = self.node(T::ElseIf);
                self.if_part(else_if)?;
                self.attach(n, P::ElseIf, false, else_if);
            } else {
                self.child(n, P::ElseBlock, Self::action_block)?;
            }
        }
        Ok(())
    }

    fn action_block(&mut self) -> R<NodeId> {
        let n = self.node(T::ActionBlock);
        if self.eat("action") {
            self.identification(n)?;
            self.typings_and_relations(n)?;
        }
        self.body(n)?;
        Ok(n)
    }

    fn effect(&mut self) -> R<NodeId> {
        let n = self.node(T::Effect);
        if self.eat("action") {
            self.identification(n)?;
            self.typings_and_relations(n)?;
            if self.at_any(&["send", "assign", "accept", "perform"]) {
                self.effect_part(n)?;
            }
        } else {
            self.effect_part(n)?;
        }
        if self.eat("{") {
            while !self.eat("}") {
                if self.annotation_ahead() {
                    self.add(n, P::Annotations, Self::annotating_element)?;
                } else {
                    self.add(n, P::Members, Self::member)?;
                }
            }
        }
        Ok(n)
    }

    fn effect_part(&mut self, n: NodeId) -> R<()> {
        if self.at("send") {
            self.send_part(n)
        } else if self.at("assign") {
            self.assign_part(n)
        } else if self.eat("accept") {
            self.accept_part(n)
        } else {
            self.expect("perform")?;
            self.text(n, P::Performed, Self::feature_chain)
        }
    }

    fn connector_part(&mut self, n: NodeId) -> R<()> {
        if !self.eat("(") {
            return self.connector_ends(n);
        }
        self.add(n, P::Ends, Self::connector_end)?;
        self.expect(",")?;
        loop {
            self.add(n, P::Ends, Self::connector_end)?;
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")")
    }

    fn connector_end(&mut self) -> R<NodeId> {
        let n = self.node(T::ConnectorEnd);
        if self.at("[") {
            self.child(n, P::Multiplicity, Self::multiplicity)?;
        }
        if self.name_at(0) && (self.at_n(1, "::>") || self.at_n(1, "references")) {
            self.text(n, P::Name, Self::name)?;
            self.pos += 1;
        }
        self.add_reference(n, P::Segments, Self::qualified_name)?;
        while self.eat(".") {
            self.add_reference(n, P::Segments, Self::name)?;
        }
        Ok(n)
    }

    // ------------------------------------------------------------ Parts of a declaration

    fn identification(&mut self, n: NodeId) -> R<()> {
        if self.eat("<") {
            self.text(n, P::ShortName, Self::name)?;
            self.expect(">")?;
        }
        if self.name_at(0) {
            self.text(n, P::Name, Self::name)?;
        }
        Ok(())
    }

    fn at_relation(&self) -> bool {
        self.at_any(&[
            ":>",
            "subsets",
            ":>>",
            "redefines",
            "::>",
            "references",
            "=>",
            "crosses",
            "[",
            "ordered",
            "nonunique",
        ])
    }

    /// `FeatureRelation*`
    fn relations(&mut self, n: NodeId) -> R<()> {
        while self.at_relation() {
            self.feature_relation(n)?;
        }
        Ok(())
    }

    fn feature_relation(&mut self, n: NodeId) -> R<()> {
        match self.tok(0).text {
            ":>" | "subsets" => {
                self.pos += 1;
                self.separated(n, P::Subsets, Self::add_chain)
            }
            ":>>" | "redefines" => {
                self.pos += 1;
                self.separated(n, P::Redefines, Self::add_chain)
            }
            "::>" | "references" => {
                self.pos += 1;
                self.add_chain(n, P::References)
            }
            "=>" | "crosses" => {
                self.pos += 1;
                self.add_chain(n, P::Crosses)
            }
            "[" => self.add(n, P::Multiplicities, Self::multiplicity),
            _ => {
                self.pos += 1;
                Ok(())
            }
        }
    }

    fn at_typing(&self) -> bool {
        self.at(":") || (self.at("defined") && self.at_n(1, "by"))
    }

    /// Moves past `:` or `defined by`, if that is what is next.
    fn typing_keyword(&mut self) -> bool {
        let found = self.at_typing();
        if found {
            self.pos += if self.at(":") { 1 } else { 2 };
        }
        found
    }

    fn written_typing(&mut self, n: NodeId) -> R<()> {
        self.typing_keyword();
        self.add_type_name(n, P::Types)?;
        self.more_types(n)
    }

    fn more_types(&mut self, n: NodeId) -> R<()> {
        while self.eat(",") {
            self.add_type_name(n, P::Types)?;
        }
        Ok(())
    }

    /// `(WrittenTyping | FeatureRelation)*`
    fn typings_and_relations(&mut self, n: NodeId) -> R<()> {
        loop {
            if self.at_typing() {
                self.written_typing(n)?;
            } else if self.at_relation() {
                self.feature_relation(n)?;
            } else {
                return Ok(());
            }
        }
    }

    fn at_value(&self) -> bool {
        self.at_any(&["=", ":=", "default"])
    }

    /// `ValuePart?`
    fn value_part(&mut self, n: NodeId) -> R<()> {
        if !self.at_value() {
            return Ok(());
        }
        if self.flag_if("default", n, P::IsDefault) {
            if !self.eat("=") {
                self.flag_if(":=", n, P::IsInitial);
            }
        } else if !self.eat("=") {
            self.flag_if(":=", n, P::IsInitial);
        }
        self.child(n, P::Value, Self::expression)
    }

    fn multiplicity(&mut self) -> R<NodeId> {
        let n = self.node(T::Multiplicity);
        self.expect("[")?;
        self.text(n, P::Lower, Self::multiplicity_bound)?;
        if self.eat("..") {
            self.text(n, P::Upper, Self::multiplicity_bound)?;
        }
        self.expect("]")?;
        Ok(n)
    }

    fn multiplicity_bound(&mut self) -> R<String> {
        if self.tok(0).kind == Kind::Number {
            let text = self.tok(0).text;
            self.pos += 1;
            return Ok(text.to_string());
        }
        if self.eat("*") {
            return Ok("*".to_string());
        }
        self.qualified_name()
    }

    // ------------------------------------------------------------ Names

    fn name(&mut self) -> R<String> {
        let t = self.tok(0);
        let value = match t.kind {
            Kind::Id => t.text.to_string(),
            Kind::Unrestricted => unquote(t.text),
            _ => return self.fail("a name"),
        };
        self.pos += 1;
        Ok(value)
    }

    fn qualified_name(&mut self) -> R<String> {
        let mut value = self.name()?;
        while self.eat("::") {
            value.push_str("::");
            value.push_str(&self.name()?);
        }
        Ok(value)
    }

    fn feature_chain(&mut self) -> R<String> {
        let mut value = self.qualified_name()?;
        while self.eat(".") {
            value.push('.');
            value.push_str(&self.name()?);
        }
        Ok(value)
    }

    fn chained_name(&mut self) -> R<String> {
        let mut value = self.name()?;
        while self.at("::") || self.at(".") {
            value.push_str(self.tok(0).text);
            self.pos += 1;
            value.push_str(&self.name()?);
        }
        Ok(value)
    }

    fn type_name(&mut self) -> R<String> {
        let conjugated = self.eat("~");
        let name = self.qualified_name()?;
        Ok(if conjugated { format!("~{name}") } else { name })
    }

    // ------------------------------------------------------------ Expressions

    fn expression(&mut self) -> R<NodeId> {
        if !self.at("if") {
            return self.operator_expression();
        }
        let n = self.node(T::IfExpression);
        self.pos += 1;
        self.child(n, P::Condition, Self::expression)?;
        self.expect("?")?;
        self.child(n, P::Then, Self::expression)?;
        self.expect("else")?;
        self.child(n, P::Else, Self::expression)?;
        Ok(n)
    }

    fn operator_expression(&mut self) -> R<NodeId> {
        let mut parts = vec![self.unary_expression()?];
        let mut operators = Vec::new();
        loop {
            let t = self.tok(0);
            if t.kind != Kind::Word || !OPERATORS.iter().any(|level| level.contains(&t.text)) {
                break;
            }
            self.pos += 1;
            operators.push(t);
            parts.push(self.unary_expression()?);
        }
        Ok(self.tree_of(&parts, &operators))
    }

    /// The tree of `a + b * c`: the root is the loosest operator, the rightmost of
    /// its level unless the level associates to the right.
    fn tree_of(&mut self, parts: &[NodeId], operators: &[Token<'a>]) -> NodeId {
        if operators.is_empty() {
            return parts[0];
        }
        let mut root = 0;
        let mut loosest = 0;
        for (i, operator) in operators.iter().enumerate() {
            let level = precedence(operator.text);
            if i == 0 || level > loosest || (level == loosest && level != 0) {
                root = i;
                loosest = level;
            }
        }
        let offset = self.ast[parts[0]].offset;
        let n = self.ast.add(T::OperatorExpression, offset);
        let left = self.tree_of(&parts[..=root], &operators[..root]);
        self.attach(n, P::Left, false, left);
        let t = operators[root];
        self.set(n, P::Operator, false, t.start as u32, Value::Text(t.text.to_string()));
        let right = self.tree_of(&parts[root + 1..], &operators[root + 1..]);
        self.attach(n, P::Right, false, right);
        n
    }

    fn unary_expression(&mut self) -> R<NodeId> {
        if self.at_any(&["+", "-", "~", "not"]) {
            let n = self.node(T::PrefixExpression);
            self.keyword(n, P::Operator);
            self.child(n, P::Operand, Self::unary_expression)?;
            return Ok(n);
        }
        if self.at_any(&["istype", "hastype", "@", "@@", "as", "meta"]) {
            let n = self.node(T::ClassificationExpression);
            self.keyword(n, P::Operator);
            self.text(n, P::Type, Self::qualified_name)?;
            return Ok(n);
        }
        if self.at("all") {
            let n = self.node(T::ExtentExpression);
            self.pos += 1;
            self.text(n, P::Type, Self::qualified_name)?;
            return Ok(n);
        }
        self.primary_expression()
    }

    fn primary_expression(&mut self) -> R<NodeId> {
        let mut current = self.base_expression()?;
        loop {
            let offset = self.ast[current].offset;
            let n;
            if self.at(".") && self.name_at(1) {
                n = self.ast.add(T::FeatureChainExpression, offset);
                self.pos += 1;
                self.text(n, P::Member, Self::name)?;
                self.invocation(n)?;
            } else if self.at(".") && self.at_n(1, "metadata") {
                n = self.ast.add(T::MetadataAccessExpression, offset);
                self.pos += 2;
            } else if self.at(".") && self.at_n(1, "{") {
                n = self.ast.add(T::CollectExpression, offset);
                self.pos += 1;
                self.child(n, P::Body, Self::body_expression)?;
            } else if self.eat(".?") {
                n = self.ast.add(T::SelectExpression, offset);
                self.child(n, P::Body, Self::body_expression)?;
            } else if self.eat("->") {
                n = self.ast.add(T::InvocationOperation, offset);
                self.text(n, P::Function, Self::qualified_name)?;
                if self.at("{") {
                    self.child(n, P::Body, Self::body_expression)?;
                } else if self.at("(") {
                    self.argument_list(n)?;
                } else {
                    self.text(n, P::Reference, Self::qualified_name)?;
                }
            } else if self.eat("[") {
                n = self.ast.add(T::BracketExpression, offset);
                if !self.at("]") {
                    self.separated(n, P::Index, Self::add_expression)?;
                }
                self.expect("]")?;
            } else if self.at("#") && self.at_n(1, "(") {
                n = self.ast.add(T::IndexExpression, offset);
                self.pos += 2;
                self.separated(n, P::Index, Self::add_expression)?;
                self.expect(")")?;
            } else {
                return Ok(current);
            }
            self.attach(n, P::Source, false, current);
            current = n;
        }
    }

    fn invocation(&mut self, n: NodeId) -> R<()> {
        if self.flag_if("(", n, P::Invocation) {
            if !self.at(")") {
                self.arguments(n)?;
            }
            self.expect(")")?;
        }
        Ok(())
    }

    fn base_expression(&mut self) -> R<NodeId> {
        let t = self.tok(0);
        let literal = match t.kind {
            Kind::Str => Some(string_value(t.text)),
            Kind::Number => Some(t.text.to_string()),
            Kind::Word if matches!(t.text, "true" | "false" | "null" | "*") => Some(t.text.to_string()),
            _ => None,
        };
        if let Some(value) = literal {
            let n = self.node(T::LiteralExpression);
            self.pos += 1;
            self.set(n, P::Value, false, t.start as u32, Value::Text(value));
            return Ok(n);
        }
        if self.at(".") && self.tok(1).kind == Kind::Number {
            let n = self.node(T::LiteralExpression);
            let value = format!(".{}", self.tok(1).text);
            self.pos += 2;
            self.set(n, P::Value, false, t.start as u32, Value::Text(value));
            return Ok(n);
        }
        if self.at("new") {
            let n = self.node(T::ConstructorExpression);
            self.pos += 1;
            self.text(n, P::Type, Self::qualified_name)?;
            self.argument_list(n)?;
            return Ok(n);
        }
        if self.name_at(0) {
            let n = self.node(T::FeatureReferenceExpression);
            self.text(n, P::Name, Self::qualified_name)?;
            self.invocation(n)?;
            return Ok(n);
        }
        if self.at("(") {
            let n = self.node(T::SequenceExpression);
            self.pos += 1;
            if !self.at(")") {
                loop {
                    self.add(n, P::Items, Self::expression)?;
                    if !self.eat(",") || self.at(")") {
                        break;
                    }
                }
            }
            self.expect(")")?;
            return Ok(n);
        }
        self.fail("an expression")
    }

    fn body_expression(&mut self) -> R<NodeId> {
        let n = self.node(T::BodyExpression);
        self.expect("{")?;
        while !self.eat("}") {
            if !self.member_or_result(n)? {
                break;
            }
        }
        Ok(n)
    }

    fn argument_list(&mut self, n: NodeId) -> R<()> {
        self.expect("(")?;
        if !self.at(")") {
            self.arguments(n)?;
        }
        self.expect(")")
    }

    fn arguments(&mut self, n: NodeId) -> R<()> {
        loop {
            self.add(n, P::Arguments, Self::argument)?;
            if !self.eat(",") {
                return Ok(());
            }
        }
    }

    fn argument(&mut self) -> R<NodeId> {
        let n = self.node(T::Argument);
        if self.name_at(0) && self.at_n(1, "=") {
            self.text(n, P::Name, Self::name)?;
            self.pos += 1;
        }
        self.child(n, P::Value, Self::expression)?;
        Ok(n)
    }
}
