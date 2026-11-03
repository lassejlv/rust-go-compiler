// Package borrowck implements rustc's borrow checking (rustc_borrowck) on a
// MIR-like control-flow graph built from the type-checked AST
// (rustc_mir_build):
//
//   - initialization and moves: use of moved or uninitialized values
//     (E0381, E0382), moves out of borrowed content (E0507, E0508),
//     reassignment of immutable locals (E0384);
//   - mutability: mutable borrows / assignments through immutable places
//     (E0596, E0594);
//   - loans (non-lexical lifetimes): conflicting accesses to borrowed
//     places while the borrow is live (E0499, E0502, E0503, E0505, E0506),
//     borrowed locals going out of scope (E0597, E0515, E0716).
//
// Types carry no lifetimes after type checking, so regions are modeled by
// loan sets attached to values (by reference depth) and propagated through
// calls according to the callee's declared signature. Whenever the model
// is unsure it drops loans: that can miss errors but never reports errors
// for valid programs.
package borrowck

import (
	"fmt"
	"strings"

	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// LocalID indexes Body.Locals. Local 0 is the return place.
type LocalID int

// DerefKind classifies a dereference by the type being dereferenced.
type DerefKind uint8

const (
	DRef    DerefKind = iota // &T
	DRefMut                  // &mut T
	DBox                     // Box<T> (owned)
	DRaw                     // *const T
	DRawMut                  // *mut T
)

// ProjKind is a place projection.
type ProjKind uint8

const (
	PDeref ProjKind = iota
	PField          // field Field of variant Variant-1 (Variant 0: struct/tuple)
	PIndex          // [Index] (Index < 0: constant / unknown index)
)

type Proj struct {
	Kind    ProjKind
	Deref   DerefKind
	Field   int
	Variant int
	Index   LocalID
	Name    string // field name, for diagnostics
}

// Place is a local followed by projections.
type Place struct {
	Local LocalID
	Proj  []Proj
}

func (p Place) project(pr Proj) Place {
	np := make([]Proj, len(p.Proj)+1)
	copy(np, p.Proj)
	np[len(p.Proj)] = pr
	return Place{Local: p.Local, Proj: np}
}

// OpKind classifies operands.
type OpKind uint8

const (
	OConst OpKind = iota
	OCopy
	OMove
)

type Operand struct {
	Kind  OpKind
	Place Place
	Ty    Ty // the operand's type when known (nil: unknown)
}

// RvKind classifies rvalues.
type RvKind uint8

const (
	RUse       RvKind = iota // Ops[0]
	RRef                     // &Place / &mut Place
	RRawRef                  // &raw Place (no loan)
	RAggregate               // value built from Ops (keeps their loans)
	ROther                   // computed from Ops (no loans: arithmetic, casts to scalars, ...)
)

type Rvalue struct {
	Kind  RvKind
	Ops   []Operand
	Place Place
	Mut   bool
}

// StmtKind classifies statements.
type StmtKind uint8

const (
	SAssign      StmtKind = iota
	SStorageDead          // local goes out of scope
	SDrop                 // drop of a place (at scope end / before reassignment)
	SFakeRead             // a read without a use of the value (match scrutinee, let _ = ..)
	SNop
)

type Stmt struct {
	Kind  StmtKind
	Place Place
	Rv    Rvalue
	Span  lexer.Span
	// Init marks the initialization of a pattern binding (the second
	// assignment of by-value bindings of guarded match arms is not a
	// reassignment).
	Init bool
}

// TermKind classifies block terminators.
type TermKind uint8

const (
	TGoto TermKind = iota
	TSwitch
	TCall
	TReturn
	TUnreachable
)

type Term struct {
	Kind    TermKind
	Targets []int // TGoto: [target]; TSwitch: successors; TCall: [return target] (none if diverging)
	Args    []Operand
	Dest    Place
	Flow    *Flow // TCall: how loans flow from arguments to the result
	Span    lexer.Span
	Name    string
}

type Block struct {
	Stmts []Stmt
	Term  Term
}

// LocalKind classifies locals.
type LocalKind uint8

const (
	LReturn LocalKind = iota
	LArg
	LVar   // user variable
	LTemp  // compiler temporary
	LUpvar  // closure capture
	LStatic // promoted constant (rvalue static promotion)
)

type LocalDecl struct {
	Kind LocalKind
	Ty   Ty
	Mut  bool
	Name string
	Span lexer.Span
	User *typeck.Local
	// Upvars: captured by reference (the local holds &T / &mut T).
	UpvarByRef bool
}

// Body is the MIR of one function or closure body.
type Body struct {
	Name   string
	Locals []*LocalDecl
	Blocks []*Block
	Span   lexer.Span
	// ClosureKind for closure bodies (-1 for functions): 0 Fn, 1 FnMut,
	// 2 FnOnce.
	ClosureKind int
}

// ---------------------------------------------------------------------------
// Debug printing

func (b *Body) placeString(p Place) string {
	s := fmt.Sprintf("_%d", p.Local)
	for _, pr := range p.Proj {
		switch pr.Kind {
		case PDeref:
			s = "(*" + s + ")"
		case PField:
			if pr.Variant > 0 {
				s = fmt.Sprintf("(%s as v%d).%d", s, pr.Variant-1, pr.Field)
			} else {
				s = fmt.Sprintf("%s.%d", s, pr.Field)
			}
		case PIndex:
			if pr.Index < 0 {
				s += "[?]"
			} else {
				s += fmt.Sprintf("[_%d]", pr.Index)
			}
		}
	}
	return s
}

func (b *Body) opString(o Operand) string {
	switch o.Kind {
	case OCopy:
		return "copy " + b.placeString(o.Place)
	case OMove:
		return "move " + b.placeString(o.Place)
	}
	return "const"
}

func (b *Body) String() string {
	var sb strings.Builder
	fmt.Fprintf(&sb, "fn %s {\n", b.Name)
	for i, l := range b.Locals {
		fmt.Fprintf(&sb, "  let _%d: %s; // %s mut=%v kind=%d\n", i, l.Ty, l.Name, l.Mut, l.Kind)
	}
	for i, bl := range b.Blocks {
		fmt.Fprintf(&sb, " bb%d:\n", i)
		for _, s := range bl.Stmts {
			switch s.Kind {
			case SAssign:
				var rv string
				switch s.Rv.Kind {
				case RUse:
					rv = b.opString(s.Rv.Ops[0])
				case RRef:
					m := ""
					if s.Rv.Mut {
						m = "mut "
					}
					rv = "&" + m + b.placeString(s.Rv.Place)
				case RRawRef:
					rv = "&raw " + b.placeString(s.Rv.Place)
				default:
					var ops []string
					for _, o := range s.Rv.Ops {
						ops = append(ops, b.opString(o))
					}
					k := "agg"
					if s.Rv.Kind == ROther {
						k = "op"
					}
					rv = k + "(" + strings.Join(ops, ", ") + ")"
				}
				fmt.Fprintf(&sb, "    %s = %s;\n", b.placeString(s.Place), rv)
			case SStorageDead:
				fmt.Fprintf(&sb, "    StorageDead(%s);\n", b.placeString(s.Place))
			case SDrop:
				fmt.Fprintf(&sb, "    drop(%s);\n", b.placeString(s.Place))
			case SFakeRead:
				fmt.Fprintf(&sb, "    FakeRead(%s);\n", b.placeString(s.Place))
			}
		}
		t := bl.Term
		switch t.Kind {
		case TGoto:
			fmt.Fprintf(&sb, "    goto bb%d;\n", t.Targets[0])
		case TSwitch:
			fmt.Fprintf(&sb, "    switch -> %v;\n", t.Targets)
		case TCall:
			var ops []string
			for _, o := range t.Args {
				ops = append(ops, b.opString(o))
			}
			fmt.Fprintf(&sb, "    %s = %s(%s) -> %v;\n", b.placeString(t.Dest), t.Name, strings.Join(ops, ", "), t.Targets)
		case TReturn:
			sb.WriteString("    return;\n")
		case TUnreachable:
			sb.WriteString("    unreachable;\n")
		}
	}
	sb.WriteString("}\n")
	return sb.String()
}
