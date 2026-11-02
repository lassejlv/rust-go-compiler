package codegen

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

type dropEntry struct {
	place string
	flag  string
	ty    Ty
	local *typeck.Local
}

type scope struct {
	drops []dropEntry
}

type loopInfo struct {
	label    string
	brk      string
	cont     string
	result   string // C variable receiving break values ("" if none)
	resultTy Ty
	depth    int // scope depth at loop entry (scopes above are exited by break)
	contDepth int
	isBlock  bool
}

type fnGen struct {
	g        *Gen
	r        *typeck.Results
	b        strings.Builder
	ind      int
	scopes   []*scope
	loops    []*loopInfo
	closure  *typeck.ClosureInfo
	capIndex map[*typeck.Local]int
	retTy    Ty
	declared map[*typeck.Local]bool
	// split records locals with per-field drop flags (partial moves).
	split map[*typeck.Local]map[int]bool
	// deferInits collects drop-flag initializations of match bindings until
	// the arm's guard has succeeded.
	deferInits *[]string
}

func newFnGen(g *Gen, r *typeck.Results) *fnGen {
	return &fnGen{g: g, r: r, ind: 1, declared: map[*typeck.Local]bool{}}
}

func (fc *fnGen) line(format string, args ...any) {
	for i := 0; i < fc.ind; i++ {
		fc.b.WriteString("  ")
	}
	fmt.Fprintf(&fc.b, format, args...)
	fc.b.WriteByte('\n')
}

func (fc *fnGen) newName(prefix string) string {
	fc.g.strN++
	return fmt.Sprintf("%s%d", prefix, fc.g.strN)
}

func (fc *fnGen) ty(e ast.Expr) Ty  { return fc.r.NodeTypes[e.GetID()] }
func (fc *fnGen) aty(e ast.Expr) Ty { return fc.r.AdjustedType(e) }

func (fc *fnGen) ctype(t Ty) string {
	fc.g.defineByValue(t)
	return fc.g.ctype(t)
}

func isZST(t Ty) bool {
	return t.Kind == TNever || t.Kind == TFnDef || (t.Kind == TTuple && len(t.Elems) == 0)
}

func (fc *fnGen) unit() string { return "((Unit){})" }

// dummy returns a placeholder value of type t (for unreachable code).
func (fc *fnGen) dummy(t Ty) string {
	return fmt.Sprintf("((%s){})", fc.ctype(t))
}

// ---------------------------------------------------------------------------
// Scopes and drops

func (fc *fnGen) pushScope() { fc.scopes = append(fc.scopes, &scope{}) }

func (fc *fnGen) popScope() {
	s := fc.scopes[len(fc.scopes)-1]
	fc.emitScopeDrops(s)
	fc.scopes = fc.scopes[:len(fc.scopes)-1]
}

func (fc *fnGen) popScopeNoDrop() { fc.scopes = fc.scopes[:len(fc.scopes)-1] }

func (fc *fnGen) emitScopeDrops(s *scope) {
	for i := len(s.drops) - 1; i >= 0; i-- {
		d := s.drops[i]
		fc.line("%s", fc.dropCode(d))
	}
	if len(s.drops) > 0 {
		fc.line("RG_CS_POP(%d);", len(s.drops))
	}
}

// dropCode renders the conditional drop of an entry.
func (fc *fnGen) dropCode(d dropEntry) string {
	if d.local != nil && len(fc.split[d.local]) > 0 {
		var parts []string
		base := fc.flagName(d.local)
		fts := fc.fieldTys(d.ty)
		for i, ft := range fts {
			if !fc.g.needsDrop(ft) {
				continue
			}
			if fc.split[d.local][i] {
				f := fmt.Sprintf("%s_%d", base, i)
				parts = append(parts, fmt.Sprintf("if (%s) { %s = 0; %s(&%s.f%d); }", f, f, fc.g.dropFn(ft), d.place, i))
			} else {
				parts = append(parts, fmt.Sprintf("if (%s) %s(&%s.f%d);", base, fc.g.dropFn(ft), d.place, i))
			}
		}
		parts = append(parts, base+" = 0;")
		return strings.Join(parts, " ")
	}
	return fmt.Sprintf("if (%s) { %s = 0; %s(&%s); }", d.flag, d.flag, fc.g.dropFn(d.ty), d.place)
}

// fieldTys returns the field types of a struct or tuple type.
func (fc *fnGen) fieldTys(t Ty) []Ty {
	if t.Kind == TTuple {
		return t.Elems
	}
	return fc.g.fieldTypes(t, 0)
}

// splittable reports whether a local's type can have per-field flags.
func (fc *fnGen) splittable(t Ty) bool {
	if t.Kind == TTuple {
		return true
	}
	return t.Kind == TAdt && t.Def.Adt.Kind == AdtStruct && fc.g.dropImpl(t) == nil && t.Def.LangItem != "manually_drop"
}

// computeSplit finds locals whose fields are accessed by value-able paths
// and gives them per-field drop flags.
func (fc *fnGen) computeSplit(body any) {
	fc.split = map[*typeck.Local]map[int]bool{}
	ast.Walk(body, func(n any) bool {
		fe, ok := n.(*ast.FieldExpr)
		if !ok {
			return true
		}
		base := unparen(fe.X)
		pe, ok := base.(*ast.PathExpr)
		if !ok || len(fc.r.Adjust[fe.X.GetID()]) > 0 {
			return true
		}
		vr := fc.r.Paths[pe.ID]
		if vr == nil || vr.Kind != typeck.VLocal {
			return true
		}
		l := vr.Local
		if !fc.g.needsDrop(l.Ty) || !fc.splittable(l.Ty) {
			return true
		}
		ft := fc.r.NodeTypes[fe.ID]
		if ft == nil || !fc.g.needsDrop(ft) {
			return true
		}
		fr, ok := fc.r.Fields[fe.ID]
		if !ok {
			return true
		}
		if fc.split[l] == nil {
			fc.split[l] = map[int]bool{}
		}
		fc.split[l][fr.Index] = true
		return true
	})
}

// allFlags returns "df = df_0 = ..." for chained assignment.
func (fc *fnGen) allFlags(l *typeck.Local) string {
	s := fc.flagName(l)
	if len(fc.split[l]) == 0 {
		return s
	}
	for i := range fc.fieldTys(l.Ty) {
		if fc.split[l][i] {
			s += fmt.Sprintf(" = %s_%d", fc.flagName(l), i)
		}
	}
	return s
}

// emitDropsFrom emits drops for scopes[depth:] (innermost first).
func (fc *fnGen) emitDropsFrom(depth int) {
	for i := len(fc.scopes) - 1; i >= depth; i-- {
		fc.emitScopeDrops(fc.scopes[i])
	}
}

func (fc *fnGen) addDrop(place, flag string, t Ty) {
	s := fc.scopes[len(fc.scopes)-1]
	s.drops = append(s.drops, dropEntry{place: place, flag: flag, ty: t})
}

// ---------------------------------------------------------------------------
// Locals

func (fc *fnGen) localName(l *typeck.Local) string { return fmt.Sprintf("l%d", l.ID) }
func (fc *fnGen) flagName(l *typeck.Local) string  { return fmt.Sprintf("df%d", l.ID) }

func (fc *fnGen) declLocal(l *typeck.Local) {
	if fc.declared[l] {
		return
	}
	fc.declared[l] = true
	fc.line("%s %s;", fc.ctype(l.Ty), fc.localName(l))
	if fc.g.needsDrop(l.Ty) {
		fc.line("uint8_t %s = 0;", fc.flagName(l))
		for i := range fc.split[l] {
			fc.line("uint8_t %s_%d = 0;", fc.flagName(l), i)
		}
		s := fc.scopes[len(fc.scopes)-1]
		s.drops = append(s.drops, dropEntry{place: fc.localName(l), flag: fc.flagName(l), ty: l.Ty, local: l})
		fc.line("RG_CS_PUSH(&%s, %s, &%s);", fc.localName(l), fc.g.dropFn(l.Ty), fc.flagName(l))
	}
}

func (fc *fnGen) markInit(l *typeck.Local) {
	if fc.g.needsDrop(l.Ty) {
		if fc.deferInits != nil {
			*fc.deferInits = append(*fc.deferInits, fmt.Sprintf("%s = 1;", fc.allFlags(l)))
			return
		}
		fc.line("%s = 1;", fc.allFlags(l))
	}
}

// localPlace returns the place expression for a local (handling closure
// captures) and the drop flag to clear on move ("" if none).
func (fc *fnGen) localPlace(l *typeck.Local) (string, string) {
	if fc.capIndex != nil {
		if i, ok := fc.capIndex[l]; ok {
			c := fc.closure.Captures[i]
			if c.ByRef {
				return fmt.Sprintf("(*env->c%d)", i), ""
			}
			return fmt.Sprintf("env->c%d", i), ""
		}
	}
	if !fc.declared[l] {
		// e.g. a local used before its declaration was emitted (shouldn't
		// happen); declare lazily.
		fc.declLocal(l)
	}
	flag := ""
	if fc.g.needsDrop(l.Ty) {
		flag = fc.allFlags(l)
	}
	return fc.localName(l), flag
}

// ---------------------------------------------------------------------------
// Temporaries

// spill stores a value in a fresh temporary without drop responsibility.
func (fc *fnGen) spill(v string, t Ty) string {
	if isZST(t) {
		fc.line("(void)%s;", v)
		return fc.unit()
	}
	n := fc.newName("t")
	fc.line("%s %s = %s;", fc.ctype(t), n, v)
	return n
}

// materialize stores a value in a temporary that is dropped at the end of
// the current scope. Returns the place and its drop flag.
func (fc *fnGen) materialize(v string, t Ty) (string, string) {
	n := fc.newName("t")
	fc.line("%s %s = %s;", fc.ctype(t), n, v)
	if fc.g.needsDrop(t) {
		flag := "d" + n
		fc.line("uint8_t %s = 1;", flag)
		fc.addDrop(n, flag, t)
		fc.line("RG_CS_PUSH(&%s, %s, &%s);", n, fc.g.dropFn(t), flag)
		return n, flag
	}
	return n, ""
}

// ---------------------------------------------------------------------------
// Blocks and statements

// blockValue lowers a block and returns its value (coerced tail).
func (fc *fnGen) blockValue(b *ast.Block, want Ty) string {
	if want == nil {
		want = fc.r.NodeTypes[b.ID]
	}
	var res string
	if want != nil && !isZST(want) {
		res = fc.newName("b")
		fc.line("%s %s;", fc.ctype(want), res)
	}
	fc.line("{")
	fc.ind++
	fc.pushScope()
	tail := blockTail(b)
	n := len(b.Stmts)
	if tail != nil {
		n--
	}
	for _, s := range b.Stmts[:n] {
		fc.stmt(s)
	}
	if tail != nil {
		v := fc.value(tail)
		tt := fc.aty(tail)
		if res != "" && tt.Kind != TNever {
			fc.line("%s = %s;", res, v)
		} else if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
	}
	fc.popScope()
	fc.ind--
	fc.line("}")
	if res == "" {
		return fc.unit()
	}
	return res
}

func blockTail(b *ast.Block) ast.Expr {
	if len(b.Stmts) == 0 {
		return nil
	}
	if es, ok := b.Stmts[len(b.Stmts)-1].(*ast.ExprStmt); ok && !es.Semi {
		return es.Expr
	}
	return nil
}

func (fc *fnGen) stmt(s ast.Stmt) {
	switch x := s.(type) {
	case *ast.LetStmt:
		fc.letStmt(x)
	case *ast.ExprStmt:
		fc.pushScope()
		v := fc.value(x.Expr)
		t := fc.aty(x.Expr)
		if t != nil && fc.g.needsDrop(t) && !isPlaceExpr(x.Expr) {
			fc.materialize(v, t)
		} else if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		fc.popScope()
	}
}

func isPlaceExpr(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.PathExpr, *ast.FieldExpr, *ast.IndexExpr:
		return true
	case *ast.UnaryExpr:
		return x.Op == ast.UnDeref
	case *ast.ParenExpr:
		return isPlaceExpr(x.X)
	}
	return false
}

func isSimple(v string) bool {
	for _, c := range v {
		if !(c == '_' || c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9') {
			return false
		}
	}
	return true
}

func (fc *fnGen) letStmt(x *ast.LetStmt) {
	pt := fc.r.NodeTypes[x.Pat.GetID()]
	if x.Init == nil {
		fc.declarePatLocals(x.Pat)
		return
	}
	// `let _ = e;` evaluates e and drops a temporary immediately.
	if _, ok := x.Pat.(*ast.WildPat); ok {
		fc.pushScope()
		if isPlaceExpr(x.Init) {
			fc.placeOp(x.Init)
		} else {
			v := fc.value(x.Init)
			if fc.g.needsDrop(fc.aty(x.Init)) {
				fc.materialize(v, fc.aty(x.Init))
			} else if v != fc.unit() {
				fc.line("(void)%s;", v)
			}
		}
		fc.popScope()
		return
	}
	if ip, ok := x.Pat.(*ast.IdentPat); ok && ip.Sub == nil && x.Else == nil {
		if l := fc.r.Locals[ip.ID]; l != nil && !l.ByRef {
			v := fc.value(x.Init)
			fc.declLocal(l)
			if !isZST(l.Ty) {
				fc.line("%s = %s;", fc.localName(l), v)
			} else if v != fc.unit() {
				fc.line("(void)%s;", v)
			}
			fc.markInit(l)
			return
		}
	}
	// general pattern: evaluate into a place, then bind (pattern derefs are
	// applied relative to the initializer's type)
	it := fc.aty(x.Init)
	var op operand
	if isPlaceExpr(x.Init) {
		op = fc.placeOp(x.Init)
	} else {
		v := fc.value(x.Init)
		p, _ := fc.materializeNoDrop(v, it)
		op = operand{s: p, place: true, ty: it}
	}
	fc.declarePatLocals(x.Pat)
	if x.Else != nil {
		test := fc.patTest(x.Pat, op.s, it)
		fc.line("if (!(%s)) {", test)
		fc.ind++
		fc.blockValue(x.Else, nil)
		fc.ind--
		fc.line("}")
	}
	if isPlaceExpr(x.Init) {
		fc.bindPatPlace(x.Pat, op.s, it, op)
	} else {
		fc.bindPat(x.Pat, op.s, it, true)
	}
	_ = pt
}

// materializeNoDrop stores a value whose ownership is transferred into
// pattern bindings.
func (fc *fnGen) materializeNoDrop(v string, t Ty) (string, string) {
	n := fc.newName("t")
	fc.line("%s %s = %s;", fc.ctype(t), n, v)
	return n, ""
}

// ---------------------------------------------------------------------------
// Returns

func (fc *fnGen) emitReturn(v string) {
	if fc.retTy == nil || isZST(fc.retTy) {
		if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		fc.emitDropsFrom(0)
		fc.line("return %s;", fc.unit())
		return
	}
	if len(fc.scopes) > 0 && !isSimple(v) {
		v = fc.spill(v, fc.retTy)
	}
	fc.emitDropsFrom(0)
	fc.line("return %s;", v)
}

// ---------------------------------------------------------------------------
// Loops

func (fc *fnGen) findLoop(label string) *loopInfo {
	for i := len(fc.loops) - 1; i >= 0; i-- {
		l := fc.loops[i]
		if label == "" {
			if l.isBlock {
				continue
			}
			return l
		}
		if l.label == label {
			return l
		}
	}
	panic("codegen: loop not found: " + label)
}

func (fc *fnGen) loopExpr(x *ast.LoopExpr, t Ty) string {
	li := &loopInfo{label: x.Label, brk: fc.newName("brk"), cont: fc.newName("cont")}
	if t != nil && !isZST(t) {
		li.result = fc.newName("lr")
		li.resultTy = t
		fc.line("%s %s;", fc.ctype(t), li.result)
	}
	li.depth = len(fc.scopes)
	fc.loops = append(fc.loops, li)
	fc.line("for (;;) {")
	fc.ind++
	fc.pushScope()
	li.contDepth = len(fc.scopes)
	fc.blockValue(x.Body, TyUnit)
	fc.popScope()
	fc.line("%s:;", li.cont)
	fc.ind--
	fc.line("}")
	fc.line("%s:;", li.brk)
	fc.loops = fc.loops[:len(fc.loops)-1]
	if li.result != "" {
		return li.result
	}
	if t != nil && t.Kind == TNever {
		return fc.unit()
	}
	return fc.unit()
}

func (fc *fnGen) whileExpr(x *ast.WhileExpr) string {
	li := &loopInfo{label: x.Label, brk: fc.newName("brk"), cont: fc.newName("cont")}
	li.depth = len(fc.scopes)
	fc.loops = append(fc.loops, li)
	fc.line("for (;;) {")
	fc.ind++
	fc.pushScope()
	li.contDepth = len(fc.scopes)
	fc.condJump(x.Cond, li.brk, li.depth)
	fc.blockValue(x.Body, TyUnit)
	fc.popScope()
	fc.line("%s:;", li.cont)
	fc.ind--
	fc.line("}")
	fc.line("%s:;", li.brk)
	fc.loops = fc.loops[:len(fc.loops)-1]
	return fc.unit()
}

func (fc *fnGen) forExpr(x *ast.ForExpr) string {
	info := fc.r.Fors[x.ID]
	fc.line("{")
	fc.ind++
	fc.pushScope()
	src := fc.value(x.Iter)
	intoIter := fc.callResolved(info.IntoIter.Def, info.IntoIter.Args, []string{src}, info.IterTy)
	it, _ := fc.materialize(intoIter, info.IterTy)
	optTy := Adt(fc.g.tcx.Lang("Option"), []Ty{info.ItemTy})
	li := &loopInfo{label: x.Label, brk: fc.newName("brk"), cont: fc.newName("cont")}
	li.depth = len(fc.scopes)
	fc.loops = append(fc.loops, li)
	fc.line("for (;;) {")
	fc.ind++
	fc.pushScope()
	li.contDepth = len(fc.scopes)
	next := fc.callResolved(info.Next.Def, info.Next.Args, []string{"&" + it}, optTy)
	n := fc.spill(next, optTy)
	someV := fc.g.tcx.Lang("Some").Variant
	fc.line("if (%s.tag != %d) goto %s;", n, someV.Disc, li.brk)
	item := fmt.Sprintf("%s.u.v%d.f0", n, someV.Index)
	fc.declarePatLocals(x.Pat)
	fc.bindPat(x.Pat, item, info.ItemTy, true)
	fc.blockValue(x.Body, TyUnit)
	fc.popScope()
	fc.line("%s:;", li.cont)
	fc.ind--
	fc.line("}")
	fc.line("%s:;", li.brk)
	fc.loops = fc.loops[:len(fc.loops)-1]
	fc.popScope()
	fc.ind--
	fc.line("}")
	return fc.unit()
}

func (fc *fnGen) breakExpr(x *ast.BreakExpr) {
	li := fc.findLoop(x.Label)
	if x.Value != nil {
		v := fc.value(x.Value)
		if li.result != "" {
			fc.line("%s = %s;", li.result, v)
		} else if v != fc.unit() {
			fc.line("(void)%s;", v)
		}
	}
	fc.emitDropsFrom(li.depth)
	fc.line("goto %s;", li.brk)
}

func (fc *fnGen) continueExpr(x *ast.ContinueExpr) {
	li := fc.findLoop(x.Label)
	fc.emitDropsFrom(li.contDepth)
	fc.line("goto %s;", li.cont)
}

// labeledBlock lowers 'a: { ... } with break-to-label support.
func (fc *fnGen) labeledBlock(x *ast.BlockExpr, t Ty) string {
	li := &loopInfo{label: x.Label, brk: fc.newName("brk"), isBlock: true}
	if t != nil && !isZST(t) {
		li.result = fc.newName("lr")
		li.resultTy = t
		fc.line("%s %s;", fc.ctype(t), li.result)
	}
	li.depth = len(fc.scopes)
	fc.loops = append(fc.loops, li)
	v := fc.blockValue(x.Block, t)
	if li.result != "" && v != fc.unit() {
		fc.line("%s = %s;", li.result, v)
	}
	fc.line("%s:;", li.brk)
	fc.loops = fc.loops[:len(fc.loops)-1]
	if li.result != "" {
		return li.result
	}
	return fc.unit()
}

// condJump lowers a condition (possibly with let-chains), jumping to
// target when it is false. Bindings are introduced in the current scope.
func (fc *fnGen) condJump(c ast.Expr, target string, dropDepth int) {
	switch x := c.(type) {
	case *ast.BinaryExpr:
		if x.Op == ast.BinAnd && containsLet(c) {
			fc.condJump(x.L, target, dropDepth)
			fc.condJump(x.R, target, dropDepth)
			return
		}
	case *ast.ParenExpr:
		if containsLet(c) {
			fc.condJump(x.X, target, dropDepth)
			return
		}
	case *ast.LetExpr:
		st := fc.r.NodeTypes[x.Pat.GetID()]
		var op operand
		if isPlaceExpr(x.Expr) {
			op = fc.placeOp(x.Expr)
		} else {
			v := fc.value(x.Expr)
			p, f := fc.materialize(v, fc.aty(x.Expr))
			op = operand{s: p, place: true, flag: f, movable: true, ty: fc.aty(x.Expr)}
		}
		_ = st
		fc.declarePatLocals(x.Pat)
		test := fc.patTest(x.Pat, op.s, fc.aty(x.Expr))
		fc.line("if (!(%s)) { %s goto %s; }", test, fc.dropsString(dropDepth), target)
		fc.bindPatPlace(x.Pat, op.s, fc.aty(x.Expr), op)
		return
	}
	v := fc.value(c)
	fc.line("if (!(%s)) { %s goto %s; }", v, fc.dropsString(dropDepth), target)
}

// dropsString renders drops for scopes[depth:] as a single line.
func (fc *fnGen) dropsString(depth int) string {
	var parts []string
	for i := len(fc.scopes) - 1; i >= depth; i-- {
		s := fc.scopes[i]
		for j := len(s.drops) - 1; j >= 0; j-- {
			d := s.drops[j]
			parts = append(parts, fc.dropCode(d))
		}
		if len(s.drops) > 0 {
			parts = append(parts, fmt.Sprintf("RG_CS_POP(%d);", len(s.drops)))
		}
	}
	return strings.Join(parts, " ")
}

func containsLet(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.LetExpr:
		return true
	case *ast.BinaryExpr:
		return x.Op == ast.BinAnd && (containsLet(x.L) || containsLet(x.R))
	case *ast.ParenExpr:
		return containsLet(x.X)
	}
	return false
}

func (fc *fnGen) ifExpr(x *ast.IfExpr, t Ty) string {
	var res string
	if t != nil && !isZST(t) && x.Else != nil {
		res = fc.newName("ir")
		fc.line("%s %s;", fc.ctype(t), res)
	}
	elseL := fc.newName("else")
	endL := fc.newName("endif")
	fc.line("{")
	fc.ind++
	depth := len(fc.scopes)
	fc.pushScope()
	fc.condJump(x.Cond, elseL, depth)
	v := fc.blockValue(x.Then, t)
	if res != "" {
		fc.line("%s = %s;", res, v)
	}
	fc.popScope()
	fc.line("goto %s;", endL)
	fc.ind--
	fc.line("}")
	fc.line("%s:;", elseL)
	if x.Else != nil {
		var ev string
		switch e := x.Else.(type) {
		case *ast.BlockExpr:
			ev = fc.blockValue(e.Block, t)
		default:
			ev = fc.coerceValue(x.Else, t)
		}
		if res != "" {
			fc.line("%s = %s;", res, ev)
		}
	}
	fc.line("%s:;", endL)
	if res != "" {
		return res
	}
	return fc.unit()
}

func (fc *fnGen) matchExpr(x *ast.MatchExpr, t Ty) string {
	var res string
	if t != nil && !isZST(t) {
		res = fc.newName("mr")
		fc.line("%s %s;", fc.ctype(t), res)
	}
	st := fc.aty(x.Scrutinee)
	var op operand
	if isPlaceExpr(x.Scrutinee) {
		op = fc.placeOp(x.Scrutinee)
		if op.unsized {
			// keep fat value
		}
	} else {
		v := fc.value(x.Scrutinee)
		p, f := fc.materialize(v, st)
		op = operand{s: p, place: true, flag: f, movable: true, ty: st}
	}
	end := fc.newName("mend")
	fc.line("{")
	fc.ind++
	for _, arm := range x.Arms {
		next := fc.newName("arm")
		fc.line("{")
		fc.ind++
		depth := len(fc.scopes)
		fc.pushScope()
		test := fc.patTest(arm.Pat, op.s, st)
		fc.line("if (!(%s)) goto %s;", test, next)
		fc.declarePatLocals(arm.Pat)
		moves := fc.bindPatDeferred(arm.Pat, op.s, st, op)
		if arm.Guard != nil {
			fc.condJump(arm.Guard, next, depth)
		}
		for _, m := range moves {
			fc.line("%s", m)
		}
		v := fc.coerceValue(arm.Body, t)
		if res != "" {
			fc.line("%s = %s;", res, v)
		} else if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		fc.popScope()
		fc.line("goto %s;", end)
		fc.ind--
		fc.line("}")
		fc.line("%s:;", next)
	}
	fc.line("rg_unreachable();")
	fc.ind--
	fc.line("}")
	fc.line("%s:;", end)
	if res != "" {
		return res
	}
	return fc.unit()
}
