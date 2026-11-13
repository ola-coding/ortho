; Written by `cargo xtask zed-grammar` from ortho's keyword list. Do not edit by hand.

; keyword.operator.logical.sysml
[
  "and"
  "or"
  "xor"
  "not"
  "implies"
] @keyword.operator

; keyword.operator.word.sysml
[
  "specializes"
  "subsets"
  "redefines"
  "references"
  "defined"
  "by"
  "of"
  "to"
  "from"
  "about"
  "via"
  "crosses"
  "hastype"
  "istype"
  "as"
  "meta"
  "all"
  "new"
] @keyword.operator

; storage.modifier.sysml
[
  "private"
  "protected"
  "public"
  "abstract"
  "variation"
  "variant"
  "individual"
  "ordered"
  "nonunique"
  "derived"
  "constant"
  "in"
  "out"
  "inout"
  "end"
  "ref"
  "default"
  "standard"
  "library"
  "parallel"
] @keyword.modifier

; keyword.control.sysml
[
  "import"
  "expose"
  "filter"
  "render"
  "accept"
  "send"
  "assign"
  "do"
  "entry"
  "exit"
  "if"
  "else"
  "then"
  "first"
  "loop"
  "while"
  "until"
  "for"
  "fork"
  "join"
  "merge"
  "decide"
  "terminate"
  "return"
  "after"
  "at"
  "when"
] @keyword.control

; keyword.other.sysml
[
  "perform"
  "exhibit"
  "include"
  "satisfy"
  "verify"
  "assert"
  "assume"
  "require"
  "allocate"
  "bind"
  "connect"
  "frame"
  "language"
  "locale"
] @keyword

; The kinds of element, as `def` and `class` in Python
[
  "action"
  "actor"
  "alias"
  "allocation"
  "analysis"
  "attribute"
  "binding"
  "calc"
  "case"
  "comment"
  "concern"
  "connection"
  "constraint"
  "def"
  "dependency"
  "doc"
  "enum"
  "event"
  "flow"
  "interface"
  "item"
  "message"
  "metadata"
  "objective"
  "occurrence"
  "package"
  "part"
  "port"
  "rendering"
  "rep"
  "requirement"
  "snapshot"
  "stakeholder"
  "state"
  "subject"
  "succession"
  "timeslice"
  "transition"
  "use"
  "verification"
  "view"
  "viewpoint"
] @keyword

; The name a definition declares, as a class's: part def Motor.
(action_definition (identification (name) @type))
(allocation_definition (identification (name) @type))
(analysis_definition (identification (name) @type))
(attribute_definition (identification (name) @type))
(calc_definition (identification (name) @type))
(case_definition (identification (name) @type))
(concern_definition (identification (name) @type))
(connection_definition (identification (name) @type))
(constraint_definition (identification (name) @type))
(enumeration_definition (identification (name) @type))
(flow_definition (identification (name) @type))
(generic_definition (identification (name) @type))
(individual_definition (identification (name) @type))
(interface_definition (identification (name) @type))
(item_definition (identification (name) @type))
(metadata_definition (identification (name) @type))
(occurrence_definition (identification (name) @type))
(part_definition (identification (name) @type))
(port_definition (identification (name) @type))
(rendering_definition (identification (name) @type))
(requirement_definition (identification (name) @type))
(state_definition (identification (name) @type))
(use_case_definition (identification (name) @type))
(verification_definition (identification (name) @type))
(view_definition (identification (name) @type))
(viewpoint_definition (identification (name) @type))

; The name a package declares: package Power.
(package_definition (identification (name) @type @namespace))
(library_package (identification (name) @type @namespace))

; Metadata on an element, as a decorator on a Python function: #refinement.
(prefix_metadata "#" @punctuation.special (qualified_name) @function)
(metadata_usage "@" @punctuation.special (qualified_name) @function)

(boolean_literal) @boolean
(null_literal) @constant.builtin
(string_literal) @string
(integer_literal) @number
(real_literal) @number

[":>>" "::>" ":>" "::" "->" ":=" ".." "==" "!=" "<=" ">=" "??" "~"] @operator

; A note, // or //* */, is not part of the model; a /* */ comment is, as a docstring
; is part of Python.
((comment) @comment
  (#not-match? @comment "^/\\*"))
((comment) @comment.doc
  (#match? @comment.doc "^/\\*"))
(block_comment_body) @comment.doc
