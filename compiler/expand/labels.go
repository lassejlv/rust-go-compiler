package expand

import (
	"strconv"

	"rustgo/compiler/ast"
)

// Hygiene of loop labels. A label named by a macro_rules! body only
// matches a loop labeled by the same expansion, or (for a macro defined in
// a block) a loop around the macro's definition: in
//
//	macro_rules! each { ($b:block) => { 'a: for i in 0..3 { $b } } }
//	'a: loop { each!({ break 'a; }) }
//
// the `break 'a` of the caller leaves the caller's loop. Labels are
// resolved after expansion like local variables (see ast.HygInfo); the
// later passes match labels by name, so when the hygienic target of a
// `break`/`continue` is not the innermost loop with that name, the target
// loop and the jumps to it get a unique name.

type labelRib struct {
	name    string
	hyg     uint32
	def     uint32 // a local macro definition (not a label)
	barrier bool   // closure, async/const block or item: labels do not cross it
	set     func(string)
	renamed string
}

type labelUse struct {
	set        func(string)
	hyg, plain *labelRib
}

type labelResolver struct {
	stack []*labelRib
	uses  []labelUse
}

// resolveLabelHygiene resolves the labels of the crate (see above).
func resolveLabelHygiene(c *ast.Crate) {
	r := &labelResolver{}
	for _, it := range c.Items {
		ast.Walk(it, r.visit)
	}
	n := 0
	for _, u := range r.uses {
		if u.hyg == nil || u.hyg == u.plain || u.hyg.renamed != "" {
			continue
		}
		n++
		u.hyg.renamed = u.hyg.name + "#" + strconv.Itoa(n)
		u.hyg.set(u.hyg.renamed)
	}
	if n == 0 {
		return
	}
	for _, u := range r.uses {
		if u.hyg != nil && u.hyg.renamed != "" {
			u.set(u.hyg.renamed)
		}
	}
}

// with walks the nodes with rb pushed.
func (r *labelResolver) with(rb *labelRib, nodes ...any) {
	n := len(r.stack)
	if rb != nil {
		r.stack = append(r.stack, rb)
	}
	for _, x := range nodes {
		ast.Walk(x, r.visit)
	}
	r.stack = r.stack[:n]
}

func loopRib(label string, hyg uint32, set func(string)) *labelRib {
	if label == "" {
		return nil
	}
	return &labelRib{name: label, hyg: hyg, set: set}
}

// use records a jump to label: its hygienic target and the loop a lookup
// by name finds.
func (r *labelResolver) use(label string, hyg uint32, set func(string)) {
	if label == "" {
		return
	}
	u := labelUse{set: set}
	for i := len(r.stack) - 1; i >= 0; i-- {
		rb := r.stack[i]
		if rb.barrier {
			break
		}
		if rb.def != 0 {
			hyg = ast.HygStrip(hyg, rb.def)
			continue
		}
		if rb.name != label {
			continue
		}
		if u.plain == nil {
			u.plain = rb
		}
		if u.hyg == nil && rb.hyg == hyg {
			u.hyg = rb
		}
	}
	r.uses = append(r.uses, u)
}

func (r *labelResolver) visit(n any) bool {
	switch x := n.(type) {
	case *ast.Item:
		if x == nil {
			return false
		}
		n := len(r.stack)
		r.stack = append(r.stack, &labelRib{barrier: true})
		ast.WalkItemKind(x.Kind, r.visit)
		r.stack = r.stack[:n]
		return false
	case *ast.Block:
		if x == nil {
			return false
		}
		n := len(r.stack)
		for _, s := range x.Stmts {
			if es, ok := s.(*ast.EmptyStmt); ok && es.MacroDef != 0 {
				r.stack = append(r.stack, &labelRib{def: es.MacroDef})
			}
			ast.Walk(s, r.visit)
		}
		r.stack = r.stack[:n]
		return false
	case *ast.ClosureExpr:
		r.with(&labelRib{barrier: true}, x.Body)
		return false
	case *ast.BlockExpr:
		if x.Async || x.Const {
			r.with(&labelRib{barrier: true}, x.Block)
			return false
		}
		r.with(loopRib(x.Label, x.LabelHyg, func(s string) { x.Label = s }), x.Block)
		return false
	case *ast.LoopExpr:
		r.with(loopRib(x.Label, x.LabelHyg, func(s string) { x.Label = s }), x.Body)
		return false
	case *ast.WhileExpr:
		r.with(loopRib(x.Label, x.LabelHyg, func(s string) { x.Label = s }), x.Cond, x.Body)
		return false
	case *ast.ForExpr:
		ast.Walk(x.Iter, r.visit)
		r.with(loopRib(x.Label, x.LabelHyg, func(s string) { x.Label = s }), x.Pat, x.Body)
		return false
	case *ast.BreakExpr:
		r.use(x.Label, x.LabelHyg, func(s string) { x.Label = s })
		if x.Value != nil {
			ast.Walk(x.Value, r.visit)
		}
		return false
	case *ast.ContinueExpr:
		r.use(x.Label, x.LabelHyg, func(s string) { x.Label = s })
		return false
	}
	return true
}
