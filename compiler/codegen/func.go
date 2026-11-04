package codegen

import (
	"fmt"
	"sort"
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
	cs    int // cleanup stack entries pushed for it (0 = 1)
}

func csCount(ds []dropEntry) int {
	n := 0
	for _, d := range ds {
		if d.cs > 0 {
			n += d.cs
		} else {
			n++
		}
	}
	return n
}

type scope struct {
	drops []dropEntry
	// tail: the tail expression of the (non-terminating) block owning this
	// scope is being lowered; before Rust 2024 the temporaries it creates
	// belong to the enclosing temporary scope (see tempScope).
	tail bool
}

type loopInfo struct {
	label     string
	brk       string
	cont      string
	result    string // C variable receiving break values ("" if none)
	resultTy  Ty
	depth     int // scope depth at loop entry (scopes above are exited by break)
	contDepth int
	isBlock   bool
	// labeled blocks: the cleanup entries of tail temporaries created in
	// the block that belong to a scope outside it (esc), and the breaks out
	// of the block (brks), which must add the entries created after them
	// (with cleared flags) so that the cleanup stack has the same layout
	// on every path to the end of the block
	esc  []escEntry
	brks []brkSite
}

type escEntry struct {
	place, drop, flag string
	target            int // index of the scope owning it
}

type brkSite struct {
	token string // placeholder line, replaced at the end of the block
	n     int    // len(esc) at the break
}

type fnGen struct {
	g        *Gen
	r        *typeck.Results
	b        strings.Builder
	ind      int
	scopes   []*scope
	loops    []*loopInfo
	closure  *typeck.ClosureInfo
	capIndex map[*typeck.Local]int // whole-variable captures
	// pathCaps: variables captured only through places inside them
	// (disjoint field captures, see typeck.Capture.Path)
	pathCaps map[*typeck.Local]bool
	// ownedUpvars: whole by-value captures of an FnOnce body that are
	// moved out of field by field: they have per-field drop flags named
	// like a local's (see genClosure)
	ownedUpvars map[*typeck.Local]bool
	retTy       Ty
	declared    map[*typeck.Local]bool
	// split records locals with per-field drop flags (partial moves).
	split map[*typeck.Local]map[int]bool
	// nested records the deeper field paths ("1.0") of split locals that
	// have their own drop flags (moves out of `l.a.b`); every prefix of
	// such a path is tracked too.
	nested map[*typeck.Local]map[string]bool
	// deferInits collects drop-flag initializations of match bindings until
	// the arm's guard has succeeded.
	deferInits *[]string
	// trackCaller: generating a #[track_caller] fn (has __loc).
	trackCaller bool
	// inConst is set while generating a const item's initializer: every
	// shared borrow of a temporary is promoted to static storage.
	inConst bool
	// resActivate, when set, collects the statements that arm residual
	// drops scheduled by residualDrops (see bindPatDeferred).
	resActivate *[]string
	// resSrcFlag maps a residual place to the drop flag that arms it.
	resSrcFlag func(place string) string
	// patSplit: patterns that move whole top-level fields out of a local
	// (which then has per-field drop flags for them).
	patSplit map[ast.Pat]patSplitInfo
	// ext: lifetime-extended temporaries of the let statement being lowered
	ext *extCtx
	// resPats: the patterns that move parts out of a (not split) local and
	// leave the rest in it (`if let Some((a, _)) = o`); resParts: the parts
	// they leave, with drop flags declared with the local, so that they are
	// dropped where the local is (see declLocal).
	resPats  map[*typeck.Local][]ast.Pat
	resParts map[ast.Pat][]resPart
	// resQueue, when set, holds the preallocated residual parts that
	// residualDrops arms (instead of scheduling new drops); resCollect,
	// when set, collects the parts instead.
	resQueue   *[]resPart
	resCollect *[]resPart
	// hoisted: declarations emitted at the top of the C function (the
	// temporaries that outlive the C block they are created in)
	hoisted []string
	// escIndex: the index of the scope tempScope returned last
	escIndex int
}

// body returns the C body of the function: the hoisted declarations, then
// the generated statements.
func (fc *fnGen) body() string {
	if len(fc.hoisted) == 0 {
		return fc.b.String()
	}
	return "  " + strings.Join(fc.hoisted, "\n  ") + "\n" + fc.b.String()
}

// resPart is a part of a local left by a pattern moving other parts out.
type resPart struct {
	place string
	ty    Ty
	flag  string
}

// extCtx describes the temporary lifetime extension of a let statement.
type extCtx struct {
	set   map[ast.NodeID]bool // expressions whose temporary is extended
	scope *scope              // the block scope the temporaries live in
	decls []string            // their declarations, emitted before the let
	ind   int
}

type patSplitInfo struct {
	local  *typeck.Local
	fields []int
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

// edition2024 reports whether the body being generated belongs to a crate
// using Rust 2024 (which changed temporary scopes of if-let and block tails).
func (fc *fnGen) edition2024() bool {
	if fc.r == nil || fc.r.Inst.Def == nil || fc.r.Inst.Def.Crate == nil {
		return false
	}
	return fc.r.Inst.Def.Crate.Edition >= "2024"
}

func (fc *fnGen) pushScope() { fc.scopes = append(fc.scopes, &scope{}) }

func (fc *fnGen) popScope() {
	s := fc.scopes[len(fc.scopes)-1]
	fc.emitScopeDrops(s)
	fc.scopes = fc.scopes[:len(fc.scopes)-1]
}

func (fc *fnGen) popScopeNoDrop() { fc.scopes = fc.scopes[:len(fc.scopes)-1] }

// mergeScope ends the innermost scope without dropping anything: what it
// owns is dropped with the enclosing scope.
func (fc *fnGen) mergeScope() {
	s := fc.scopes[len(fc.scopes)-1]
	fc.scopes = fc.scopes[:len(fc.scopes)-1]
	p := fc.scopes[len(fc.scopes)-1]
	p.drops = append(p.drops, s.drops...)
}

func (fc *fnGen) emitScopeDrops(s *scope) {
	for i := len(s.drops) - 1; i >= 0; i-- {
		d := s.drops[i]
		fc.line("%s", fc.dropCode(d))
	}
	if len(s.drops) > 0 {
		fc.line("RG_CS_POP(%d);", csCount(s.drops))
	}
}

// dropCode renders the conditional drop of an entry.
func (fc *fnGen) dropCode(d dropEntry) string {
	if d.local != nil && len(fc.split[d.local]) > 0 {
		var parts []string
		base := fc.flagName(d.local)
		if fc.isBoxTy(d.local.Ty) {
			// a box with fields moved out of it: the remaining fields,
			// then the heap block
			fc.dropParts(&parts, d.local, "(*"+d.place+".f0)", d.local.Ty.Args[0], nil, base)
			parts = append(parts, fmt.Sprintf("if (%s) { %s }", base, fc.g.boxFree(d.place, d.local.Ty)))
		} else {
			fc.dropParts(&parts, d.local, d.place, d.ty, nil, base)
		}
		parts = append(parts, base+" = 0;")
		return strings.Join(parts, " ")
	}
	code := fmt.Sprintf("if (%s) { %s = 0; %s(&%s); }", d.flag, d.flag, fc.g.dropFn(d.ty), d.place)
	if d.local != nil && len(fc.resPats[d.local]) > 0 {
		// the parts left by a partial move out of it (in field order)
		var parts []string
		for _, p := range fc.resPats[d.local] {
			for _, rp := range fc.resParts[p] {
				parts = append(parts, fmt.Sprintf("if (%s) { %s = 0; %s(&%s); }", rp.flag, rp.flag, fc.g.dropFn(rp.ty), rp.place))
			}
		}
		code = strings.Join(append(parts, code), " ")
	}
	return code
}

// fieldTys returns the field types of a struct or tuple type (or the
// elements of an array).
func (fc *fnGen) fieldTys(t Ty) []Ty {
	if t.Kind == TTuple {
		return t.Elems
	}
	if t.Kind == TArray {
		n := int(fc.g.arrayLen(t))
		ts := make([]Ty, n)
		for i := range ts {
			ts[i] = t.Elem
		}
		return ts
	}
	return fc.g.fieldTypes(t, 0)
}

// pathKey renders a field path of a local ("1.0").
func pathKey(path []int) string {
	var b strings.Builder
	for i, f := range path {
		if i > 0 {
			b.WriteByte('.')
		}
		fmt.Fprint(&b, f)
	}
	return b.String()
}

// tracked reports whether the field path of l has its own drop flag.
func (fc *fnGen) tracked(l *typeck.Local, path []int) bool {
	switch len(path) {
	case 0:
		return true
	case 1:
		return fc.split[l][path[0]]
	}
	return fc.nested[l][pathKey(path)]
}

// trackedBelow reports whether a field path strictly inside path has its
// own drop flag.
func (fc *fnGen) trackedBelow(l *typeck.Local, path []int) bool {
	if len(path) == 0 {
		return len(fc.split[l]) > 0
	}
	k := pathKey(path) + "."
	for p := range fc.nested[l] {
		if strings.HasPrefix(p, k) {
			return true
		}
	}
	return false
}

// pathFlag is the drop flag of a tracked field path of l.
func (fc *fnGen) pathFlag(l *typeck.Local, path []int) string {
	s := fc.flagName(l)
	for _, f := range path {
		s += fmt.Sprintf("_%d", f)
	}
	return s
}

// pathFlags returns "f = f_0 = ..." (the flag of a tracked field path and
// those of the paths inside it) for chained assignment.
func (fc *fnGen) pathFlags(l *typeck.Local, path []int) string {
	s := fc.pathFlag(l, path)
	var subs []string
	if len(path) == 0 {
		for i := range fc.fieldTys(fc.splitTy(l)) {
			if fc.split[l][i] {
				subs = append(subs, fmt.Sprintf("%d", i))
			}
		}
	}
	prefix := pathKey(path) + "."
	for p := range fc.nested[l] {
		if len(path) == 0 || strings.HasPrefix(p, prefix) {
			subs = append(subs, p)
		}
	}
	sort.Strings(subs)
	for _, p := range subs {
		s += " = " + fc.flagName(l) + "_" + strings.ReplaceAll(p, ".", "_")
	}
	return s
}

// dropParts renders the drop of the parts of place (field path `path` of
// split local l, of type t) still owned: tracked parts by their own
// flags, the others by guard (the flag of the nearest tracked ancestor).
func (fc *fnGen) dropParts(parts *[]string, l *typeck.Local, place string, t Ty, path []int, guard string) {
	fts := fc.fieldTys(t)
	order := make([]int, len(fts))
	for i := range order {
		order[i] = i
	}
	if len(path) == 0 {
		order = fc.splitDropOrder(t, fc.split[l])
	}
	for _, i := range order {
		ft := fts[i]
		if !fc.g.needsDrop(ft) {
			continue
		}
		sub := append(append([]int(nil), path...), i)
		sp := place + fieldSel(t, i)
		switch {
		case fc.trackedBelow(l, sub):
			f := fc.pathFlag(l, sub)
			fc.dropParts(parts, l, sp, ft, sub, f)
			*parts = append(*parts, f+" = 0;")
		case fc.tracked(l, sub):
			f := fc.pathFlag(l, sub)
			*parts = append(*parts, fmt.Sprintf("if (%s) { %s = 0; %s(&%s); }", f, f, fc.g.dropFn(ft), sp))
		default:
			*parts = append(*parts, fmt.Sprintf("if (%s) %s(&%s);", guard, fc.g.dropFn(ft), sp))
		}
	}
}

// declSplitFlags declares the drop flags of split local l (initialized to
// init) and pushes its parts on the cleanup stack; it returns the number of
// cleanup entries pushed.
func (fc *fnGen) declSplitFlags(l *typeck.Local, place string, init int) int {
	for _, f := range strings.Split(fc.pathFlags(l, nil), " = ") {
		fc.line("uint8_t %s = %d;", f, init)
	}
	// unwinding drops the parts still owned (pushed last-first so that
	// they are dropped in order)
	var pushes []string
	box := fc.isBoxTy(l.Ty)
	// walk visits the parts (p: their C selector relative to place, or to
	// a box's contents)
	var walk func(p string, t Ty, path []int, guard string)
	walk = func(p string, t Ty, path []int, guard string) {
		fts := fc.fieldTys(t)
		order := make([]int, len(fts))
		for i := range order {
			order[i] = i
		}
		if len(path) == 0 {
			order = fc.splitDropOrder(t, fc.split[l])
		}
		for _, i := range order {
			ft := fts[i]
			if !fc.g.needsDrop(ft) {
				continue
			}
			sub := append(append([]int(nil), path...), i)
			sp := p + fieldSel(t, i)
			g := guard
			if fc.tracked(l, sub) {
				g = fc.pathFlag(l, sub)
			}
			if fc.trackedBelow(l, sub) {
				walk(sp, ft, sub, g)
				continue
			}
			if box {
				// the parts' addresses depend on the box pointer, which
				// is not initialized yet
				pushes = append(pushes, fmt.Sprintf("RG_CS_PUSH(&%s, %s, &%s);", place, fc.g.boxPartFn(l.Ty, sp, ft), g))
				continue
			}
			pushes = append(pushes, fmt.Sprintf("RG_CS_PUSH(&%s%s, %s, &%s);", place, sp, fc.g.dropFn(ft), g))
		}
	}
	n := 0
	if box {
		// the heap block of a box with fields moved out of it is freed
		// after its remaining fields are dropped
		fc.line("RG_CS_PUSH(&%s, %s, &%s);", place, fc.g.boxFreeFn(l.Ty), fc.flagName(l))
		n++
	}
	walk("", fc.splitTy(l), nil, fc.flagName(l))
	for i := len(pushes) - 1; i >= 0; i-- {
		fc.line("%s", pushes[i])
	}
	return n + len(pushes)
}

// splitTy is the type whose fields a split local's flags track: its own,
// or its contents for a box (fields moved out of `*b`).
func (fc *fnGen) splitTy(l *typeck.Local) Ty {
	if fc.isBoxTy(l.Ty) {
		return l.Ty.Args[0]
	}
	return l.Ty
}

// boxLocal returns the local of e if e is a box local dereferenced
// (implicitly when adjusted, else explicitly: `(*b)`), whose fields can be
// moved out of individually.
func (fc *fnGen) boxLocal(e ast.Expr, adjusted bool) *typeck.Local {
	if adjusted {
		adj := fc.r.Adjust[e.GetID()]
		if len(adj) != 1 || adj[0].Kind != typeck.AdjDeref {
			return nil
		}
	} else {
		u, ok := unparen(e).(*ast.UnaryExpr)
		if !ok || u.Op != ast.UnDeref || len(fc.r.Adjust[u.X.GetID()]) > 0 {
			return nil
		}
		if _, overloaded := fc.r.Callees[u.ID]; overloaded {
			return nil
		}
		e = u.X
	}
	pe, ok := unparen(e).(*ast.PathExpr)
	if !ok {
		return nil
	}
	vr := fc.r.Paths[pe.ID]
	if vr == nil || vr.Kind != typeck.VLocal || vr.Local.ByRef || !fc.isBoxTy(vr.Local.Ty) {
		return nil
	}
	if ct := vr.Local.Ty.Args[0]; ct.IsUnsized() || !fc.splittable(ct) {
		return nil
	}
	return vr.Local
}

// fieldSel is the C selector of field (or array element) i.
func fieldSel(t Ty, i int) string {
	if t.Kind == TArray {
		return fmt.Sprintf(".a[%d]", i)
	}
	return fmt.Sprintf(".f%d", i)
}

// splitDropOrder returns the order in which the fields of a split local
// are dropped: declaration order, except for arrays, which rustc splits
// into runs of elements without moves out of them and the moved-from
// elements, dropped from the last part to the first.
func (fc *fnGen) splitDropOrder(t Ty, moved map[int]bool) []int {
	n := len(fc.fieldTys(t))
	var order []int
	if t.Kind != TArray {
		for i := 0; i < n; i++ {
			order = append(order, i)
		}
		return order
	}
	var parts [][2]int
	for i := 0; i < n; {
		j := i + 1
		if !moved[i] {
			for j < n && !moved[j] {
				j++
			}
		}
		parts = append(parts, [2]int{i, j})
		i = j
	}
	for k := len(parts) - 1; k >= 0; k-- {
		for i := parts[k][0]; i < parts[k][1]; i++ {
			order = append(order, i)
		}
	}
	return order
}

// splittable reports whether a local's type can have per-field flags.
func (fc *fnGen) splittable(t Ty) bool {
	if t.Kind == TTuple {
		return true
	}
	if t.Kind == TArray {
		return t.Len.Kind == TConst && t.Len.Val <= 64
	}
	return t.Kind == TAdt && t.Def.Adt.Kind == AdtStruct && fc.g.dropImpl(t) == nil && t.Def.LangItem != "manually_drop"
}

// computeSplit finds locals whose fields are accessed by value-able paths
// and gives them per-field drop flags.
func (fc *fnGen) computeSplit(body any) {
	fc.split = map[*typeck.Local]map[int]bool{}
	fc.patSplit = map[ast.Pat]patSplitInfo{}
	fc.nested = map[*typeck.Local]map[string]bool{}
	fc.resPats = map[*typeck.Local][]ast.Pat{}
	fc.resParts = map[ast.Pat][]resPart{}
	type patSrc struct {
		p   ast.Pat
		src ast.Expr
	}
	var pats []patSrc
	defer func() {
		// after the split locals are known
		for _, ps := range pats {
			fc.residualPat(ps.p, ps.src)
		}
	}()
	ast.Walk(body, func(n any) bool {
		switch x := n.(type) {
		case *ast.StructExpr:
			fc.splitStructBase(x)
			return true
		case *ast.LetStmt:
			if x.Init != nil {
				fc.splitPatMove(x.Pat, x.Init)
				pats = append(pats, patSrc{x.Pat, x.Init})
			}
			return true
		case *ast.LetExpr:
			fc.splitPatMove(x.Pat, x.Expr)
			pats = append(pats, patSrc{x.Pat, x.Expr})
			return true
		case *ast.MatchExpr:
			for _, a := range x.Arms {
				fc.splitPatMove(a.Pat, x.Scrutinee)
				pats = append(pats, patSrc{a.Pat, x.Scrutinee})
			}
			return true
		case *ast.ClosureExpr:
			fc.splitClosureMoves(x)
			return true
		}
		fe, ok := n.(*ast.FieldExpr)
		if !ok {
			return true
		}
		if l, path := fc.fieldPath(fe); l != nil && len(path) > 1 {
			if ft := fc.r.NodeTypes[fe.ID]; ft != nil && fc.g.needsDrop(ft) {
				fc.trackPath(l, path)
			}
		} else if l != nil && fc.isBoxTy(l.Ty) {
			// a field of a box's contents (`b.x`, `(*b).x`)
			if ft := fc.r.NodeTypes[fe.ID]; ft != nil && fc.g.needsDrop(ft) {
				fc.trackPath(l, path)
			}
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

// fieldPath returns the local and field path of `l.a.b` (fields of
// structs and tuples, no dereferences), or nil.
func (fc *fnGen) fieldPath(fe *ast.FieldExpr) (*typeck.Local, []int) {
	var path []int
	var e ast.Expr = fe
	for {
		x, ok := unparen(e).(*ast.FieldExpr)
		if !ok {
			break
		}
		fr, ok := fc.r.Fields[x.ID]
		if !ok || fr.Union {
			return nil, nil
		}
		path = append([]int{fr.Index}, path...)
		if len(fc.r.Adjust[x.X.GetID()]) > 0 {
			// `b.x` with b a box: a path in its contents
			if l := fc.boxLocal(x.X, true); l != nil {
				return l, path
			}
			return nil, nil
		}
		e = x.X
	}
	if l := fc.boxLocal(e, false); l != nil {
		return l, path // `(*b).x`
	}
	pe, ok := unparen(e).(*ast.PathExpr)
	if !ok || len(fc.r.Adjust[e.GetID()]) > 0 {
		return nil, nil
	}
	vr := fc.r.Paths[pe.ID]
	if vr == nil || vr.Kind != typeck.VLocal || vr.Local.ByRef || fc.isBoxTy(vr.Local.Ty) {
		return nil, nil
	}
	return vr.Local, path
}

// trackPath gives the field path of l (and its prefixes) drop flags of
// their own, when every type along it can be split.
func (fc *fnGen) trackPath(l *typeck.Local, path []int) {
	if len(path) == 0 || !fc.g.needsDrop(l.Ty) || len(path) > 4 {
		return
	}
	t := fc.splitTy(l)
	for i, f := range path {
		if !fc.splittable(t) || (t.Kind == TArray && i > 0) {
			path = path[:i]
			break
		}
		fts := fc.fieldTys(t)
		if f >= len(fts) {
			return
		}
		t = fts[f]
	}
	if len(path) == 0 {
		return
	}
	if fc.split[l] == nil {
		fc.split[l] = map[int]bool{}
	}
	fc.split[l][path[0]] = true
	for k := 2; k <= len(path); k++ {
		if fc.nested[l] == nil {
			fc.nested[l] = map[string]bool{}
		}
		fc.nested[l][pathKey(path[:k])] = true
	}
}

// splitClosureMoves gives per-field drop flags to the locals a closure
// moves fields out of (by-value captures of `t.1`).
func (fc *fnGen) splitClosureMoves(x *ast.ClosureExpr) {
	t := fc.r.NodeTypes[x.ID]
	if t == nil || t.Kind != TClosure {
		return
	}
	ci := fc.g.tcx.Closures[t]
	if ci == nil {
		return
	}
	for _, c := range ci.Captures {
		if c.ByRef || len(c.Path) == 0 || c.Path[0].Deref || fc.g.tcx.IsCopy(c.Ty) || !fc.g.needsDrop(c.Ty) {
			continue
		}
		l := c.Local
		if l.ByRef || l.Ty.Kind == TArray {
			continue
		}
		path := make([]int, len(c.Path))
		for i, pr := range c.Path {
			if pr.Deref {
				path = nil
				break
			}
			path[i] = pr.Field
		}
		fc.trackPath(l, path)
	}
}

// splitPatMove records a pattern that moves whole top-level fields out of a
// plain local (`let (a, _) = t;`, `match s { S { a, .. } => .. }`): those
// fields get their own drop flags, and the rest stays in the local, which
// is dropped (and usable) as usual.
func (fc *fnGen) splitPatMove(p ast.Pat, src ast.Expr) {
	if len(fc.r.Adjust[src.GetID()]) > 0 || len(fc.r.PatDerefs[p.GetID()]) > 0 {
		return
	}
	pe, ok := unparen(src).(*ast.PathExpr)
	if !ok {
		return
	}
	vr := fc.r.Paths[pe.ID]
	if vr == nil || vr.Kind != typeck.VLocal || vr.Local.ByRef {
		return
	}
	l := vr.Local
	if !fc.g.needsDrop(l.Ty) || !fc.splittable(l.Ty) {
		return
	}
	var subs []ast.Pat
	var idx, rest []int
	switch x := p.(type) {
	case *ast.TuplePat:
		if l.Ty.Kind != TTuple {
			return
		}
		fc.seqFields(x.Elems, len(l.Ty.Elems), func(pi, fi int) {
			subs, idx = append(subs, x.Elems[pi]), append(idx, fi)
		})
	case *ast.TupleStructPat:
		if l.Ty.Kind != TAdt {
			return
		}
		fc.seqFields(x.Elems, len(fc.g.fieldTypes(l.Ty, 0)), func(pi, fi int) {
			subs, idx = append(subs, x.Elems[pi]), append(idx, fi)
		})
	case *ast.StructPat:
		v := fc.r.Variants[x.ID]
		if v == nil || l.Ty.Kind != TAdt {
			return
		}
		for _, f := range x.Fields {
			subs, idx = append(subs, f.Pat), append(idx, v.FieldIndex(f.Name.Name))
		}
	case *ast.SlicePat:
		if l.Ty.Kind != TArray {
			return
		}
		n := int(fc.g.arrayLen(l.Ty))
		restIdx := -1
		for i, e := range x.Elems {
			if isRestLike(e) {
				restIdx = i
			}
		}
		for pi, e := range x.Elems {
			fi := pi
			if pi == restIdx {
				ip, ok := e.(*ast.IdentPat)
				if !ok {
					continue
				}
				rl := fc.r.Locals[ip.ID]
				if rl == nil || rl.ByRef || fc.g.tcx.IsCopy(rl.Ty) {
					continue
				}
				for j := restIdx; j < n-(len(x.Elems)-restIdx-1); j++ {
					rest = append(rest, j)
				}
				continue
			}
			if restIdx >= 0 && pi > restIdx {
				fi = n - (len(x.Elems) - pi)
			}
			subs, idx = append(subs, e), append(idx, fi)
		}
	default:
		return
	}
	fields := rest
	for k, sp := range subs {
		if !fc.patMoves(sp) {
			continue
		}
		ip, ok := sp.(*ast.IdentPat)
		if !ok || ip.Sub != nil || fc.r.Paths[ip.ID] != nil || len(fc.r.PatDerefs[ip.ID]) > 0 {
			return // moves part of a field: handled by residual drops
		}
		fields = append(fields, idx[k])
	}
	if len(fields) == 0 {
		return
	}
	if fc.split[l] == nil {
		fc.split[l] = map[int]bool{}
	}
	for _, i := range fields {
		fc.split[l][i] = true
	}
	fc.patSplit[p] = patSplitInfo{local: l, fields: fields}
}

// structBaseLocal returns the local of a struct-update base `..l` that is
// moved field by field (nil if the base is not a plain droppable local).
func (fc *fnGen) structBaseLocal(se *ast.StructExpr) *typeck.Local {
	if se.Base == nil || len(fc.r.Adjust[se.Base.GetID()]) > 0 {
		return nil
	}
	pe, ok := unparen(se.Base).(*ast.PathExpr)
	if !ok {
		return nil
	}
	vr := fc.r.Paths[pe.ID]
	if vr == nil || vr.Kind != typeck.VLocal || vr.Local.ByRef {
		return nil
	}
	l := vr.Local
	if !fc.g.needsDrop(l.Ty) || !fc.splittable(l.Ty) || l.Ty.Kind != TAdt || l.Ty.Def.Adt.Kind != AdtStruct {
		return nil
	}
	return l
}

// splitStructBase gives `..l` per-field flags for the fields it moves, so
// the explicitly given fields stay in l and are dropped with it.
func (fc *fnGen) splitStructBase(se *ast.StructExpr) {
	l := fc.structBaseLocal(se)
	if l == nil {
		return
	}
	v := fc.r.Variants[se.ID]
	if v == nil {
		return
	}
	given := map[int]bool{}
	for _, f := range se.Fields {
		given[v.FieldIndex(f.Name.Name)] = true
	}
	for i, ft := range fc.fieldTys(l.Ty) {
		if !given[i] && fc.g.needsDrop(ft) {
			if fc.split[l] == nil {
				fc.split[l] = map[int]bool{}
			}
			fc.split[l][i] = true
		}
	}
}

// allFlags returns "df = df_0 = ..." for chained assignment.
func (fc *fnGen) allFlags(l *typeck.Local) string {
	if len(fc.split[l]) == 0 {
		return fc.flagName(l)
	}
	return fc.pathFlags(l, nil)
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
		s := fc.scopes[len(fc.scopes)-1]
		if len(fc.split[l]) > 0 {
			// per-field flags
			n := fc.declSplitFlags(l, fc.localName(l), 0)
			s.drops = append(s.drops, dropEntry{place: fc.localName(l), flag: fc.flagName(l), ty: l.Ty, local: l, cs: n})
			return
		}
		fc.line("uint8_t %s = 0;", fc.flagName(l))
		fc.line("RG_CS_PUSH(&%s, %s, &%s);", fc.localName(l), fc.g.dropFn(l.Ty), fc.flagName(l))
		cs := 1 + fc.declResidualParts(l)
		s.drops = append(s.drops, dropEntry{place: fc.localName(l), flag: fc.flagName(l), ty: l.Ty, local: l, cs: cs})
	}
}

// declResidualParts declares the drop flags of the parts of l that the
// patterns of resPats[l] leave in it when they move other parts out (and
// pushes them on the cleanup stack). They are armed when such a pattern
// moves out of l, and dropped with l (first, in field order). It returns
// the number of cleanup entries pushed.
func (fc *fnGen) declResidualParts(l *typeck.Local) int {
	n := 0
	for _, p := range fc.resPats[l] {
		var parts []resPart
		fc.resCollect = &parts
		fc.residualDrops(p, fc.localName(l), l.Ty, false)
		fc.resCollect = nil
		for i := range parts {
			parts[i].flag = fc.newName(fmt.Sprintf("drl%d_", l.ID))
			fc.line("uint8_t %s = 0;", parts[i].flag)
		}
		for i := len(parts) - 1; i >= 0; i-- {
			fc.line("RG_CS_PUSH(&%s, %s, &%s);", parts[i].place, fc.g.dropFn(parts[i].ty), parts[i].flag)
		}
		n += len(parts)
		fc.resParts[p] = parts
	}
	return n
}

// residualPat records p (matched against src) if it moves parts out of a
// plain local that is not split, leaving others in it.
func (fc *fnGen) residualPat(p ast.Pat, src ast.Expr) {
	if fc.patSplit[p].local != nil || len(fc.r.Adjust[src.GetID()]) > 0 || len(fc.r.PatDerefs[p.GetID()]) > 0 {
		return
	}
	pe, ok := unparen(src).(*ast.PathExpr)
	if !ok {
		return
	}
	vr := fc.r.Paths[pe.ID]
	if vr == nil || vr.Kind != typeck.VLocal || vr.Local.ByRef {
		return
	}
	l := vr.Local
	if !fc.g.needsDrop(l.Ty) || len(fc.split[l]) > 0 || !fc.patMoves(p) {
		return
	}
	if _, isIdent := p.(*ast.IdentPat); isIdent {
		return // moves the whole value
	}
	fc.resPats[l] = append(fc.resPats[l], p)
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
			if fc.ownedUpvars[l] {
				return fmt.Sprintf("env->c%d", i), fc.allFlags(l)
			}
			if fc.closure.Kind == 2 && fc.g.needsDrop(c.Ty) {
				// FnOnce body owns its by-value captures
				return fmt.Sprintf("env->c%d", i), fmt.Sprintf("dfc%d", i)
			}
			return fmt.Sprintf("env->c%d", i), ""
		}
	}
	if fc.pathCaps[l] {
		fc.g.fatalf("%s: internal error: variable `%s` is captured only partly by the closure", l.Span.Lo, l.Name)
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
	if sc, k := fc.tempScope(); sc != nil && fc.g.needsDrop(t) {
		// a temporary of a block's tail expression: it outlives the
		// block's locals
		fc.hoisted = append(fc.hoisted, fmt.Sprintf("%s %s;", fc.ctype(t), n))
		fc.line("%s = %s;", n, v)
		flag := "d" + n
		fc.hoistFlag(flag)
		sc.drops = append(sc.drops, dropEntry{place: n, flag: flag, ty: t})
		fc.escPush(k, n, fc.g.dropFn(t), flag)
		return n, flag
	}
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

// tempScope reports where a temporary created now is dropped when that is
// not the innermost scope: before Rust 2024, the temporaries of a block's
// tail expression live until the end of the enclosing temporary scope (the
// enclosing statement, or the terminating scope of an `if` branch, a loop
// or function body, a match arm, ...), so after the block's locals. It
// returns that scope and the number of cleanup-stack entries above it that
// belong to the blocks in between (the temporary's cleanup entry goes
// below them, so that unwinding drops it after their locals too), or nil.
func (fc *fnGen) tempScope() (*scope, int) {
	k := 0
	i := len(fc.scopes) - 1
	for ; i > 0 && fc.scopes[i].tail; i-- {
		k += csCount(fc.scopes[i].drops)
	}
	if i == len(fc.scopes)-1 {
		return nil, 0
	}
	fc.escIndex = i
	return fc.scopes[i], k
}

// escPush emits the cleanup entry of a temporary owned by an enclosing
// scope (see tempScope), k entries below the top.
func (fc *fnGen) escPush(k int, place, drop, flag string) {
	fc.line("RG_CS_INS(%d, &%s, %s, &%s);", k, place, drop, flag)
	for _, li := range fc.loops {
		if li.isBlock && fc.escIndex < li.depth {
			li.esc = append(li.esc, escEntry{place: place, drop: drop, flag: flag, target: fc.escIndex})
		}
	}
}

// hoistFlag declares a (cleared) drop flag at the top of the C function and
// sets it.
func (fc *fnGen) hoistFlag(flag string) {
	fc.hoisted = append(fc.hoisted, fmt.Sprintf("uint8_t %s = 0;", flag))
	fc.line("%s = 1;", flag)
}

// ---------------------------------------------------------------------------
// Blocks and statements

// blockValue lowers a block and returns its value (coerced tail). A
// terminating block (a function or loop body, an `if` branch, ...) is the
// temporary scope of its tail expression: before Rust 2024 those
// temporaries are dropped when it ends, after its locals. Other blocks
// leave them to the enclosing temporary scope.
func (fc *fnGen) blockValue(b *ast.Block, want Ty, term bool) string {
	if term && !fc.edition2024() {
		fc.pushScope()
		v := fc.blockValue(b, want, false)
		fc.popScope()
		return v
	}
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
		sc := fc.scopes[len(fc.scopes)-1]
		mark := len(sc.drops)
		sc.tail = !fc.edition2024()
		v := fc.value(tail)
		tt := fc.aty(tail)
		if res != "" && tt.Kind != TNever {
			fc.line("%s = %s;", res, v)
		} else if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		sc.tail = false
		if !fc.edition2024() && mark > 0 && len(sc.drops) > mark {
			// Before Rust 2024, temporaries of a block's tail expression
			// outlive the block's locals: drop the locals first.
			temps := append([]dropEntry(nil), sc.drops[mark:]...)
			sc.drops = append(temps, sc.drops[:mark]...)
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

// extendingExpr records the temporaries of e, an extending expression of a
// let initializer, whose lifetime is extended to the end of the enclosing
// block (rustc's rules): the operand of an extending borrow `&e` and the
// places it is projected from; the operands of extending borrows, casts,
// struct/tuple/array literals and tuple-struct constructor calls, the tails
// of extending blocks and the branches/arms of extending if/match
// expressions are extending expressions themselves. Other temporaries of
// the initializer (call arguments, method receivers, non-borrowed literal
// fields, ...) die at the end of the statement.
func (fc *fnGen) extendingExpr(e ast.Expr, set map[ast.NodeID]bool) {
	switch x := e.(type) {
	case *ast.RefExpr:
		if x.Raw {
			return
		}
		fc.extendedPlace(x.X, set)
		fc.extendingExpr(x.X, set)
	case *ast.ParenExpr:
		fc.extendingExpr(x.X, set)
	case *ast.CastExpr:
		fc.extendingExpr(x.X, set)
	case *ast.TupleExpr:
		for _, el := range x.Elems {
			fc.extendingExpr(el, set)
		}
	case *ast.ArrayExpr:
		for _, el := range x.Elems {
			fc.extendingExpr(el, set)
		}
	case *ast.StructExpr:
		for _, f := range x.Fields {
			if f.Expr != nil {
				fc.extendingExpr(f.Expr, set)
			}
		}
	case *ast.CallExpr:
		pe, ok := unparen(x.Fn).(*ast.PathExpr)
		if !ok {
			return
		}
		vr := fc.r.Paths[pe.ID]
		// std's pin! helper stands in for rustc's `Pin { pointer: &mut {..} }`
		if vr == nil || vr.Kind != typeck.VFn || (vr.Def.Kind != DefStruct && vr.Def.Kind != DefVariant && vr.Def.Name != "__pin") {
			return
		}
		for _, a := range x.Args {
			fc.extendingExpr(a, set)
		}
	case *ast.BlockExpr:
		if x.Block != nil && !x.Async && !x.Const {
			if t := blockTail(x.Block); t != nil {
				fc.extendingExpr(t, set)
			}
		}
	case *ast.IfExpr:
		// through the branches' tails, never the condition
		if x.Then != nil {
			if t := blockTail(x.Then); t != nil {
				fc.extendingExpr(t, set)
			}
		}
		if x.Else != nil {
			fc.extendingExpr(x.Else, set)
		}
	case *ast.MatchExpr:
		// likewise through the arm bodies, never the scrutinee
		for _, a := range x.Arms {
			fc.extendingExpr(a.Body, set)
		}
	}
}

// extendedPlace records that the temporary holding the value of e (if e is
// a value expression) lives until the end of the enclosing block, and so do
// those of the places a field, index or deref expression projects from.
func (fc *fnGen) extendedPlace(e ast.Expr, set map[ast.NodeID]bool) {
	set[e.GetID()] = true
	switch x := e.(type) {
	case *ast.ParenExpr:
		fc.extendedPlace(x.X, set)
	case *ast.FieldExpr:
		fc.extendedPlace(x.X, set)
	case *ast.IndexExpr:
		fc.extendedPlace(x.X, set)
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			fc.extendedPlace(x.X, set)
		}
	}
}

// materializeExt stores the value of op in a temporary whose lifetime was
// extended to the end of the block of the let statement being lowered: it
// is declared (with its drop flag and cleanup entry) before the statement
// and dropped with the block's locals.
func (fc *fnGen) materializeExt(op operand) operand {
	t := op.ty
	n := fc.newName("t")
	ind := strings.Repeat("  ", fc.ext.ind)
	fc.ext.decls = append(fc.ext.decls, fmt.Sprintf("%s%s %s;", ind, fc.ctype(t), n))
	fc.line("%s = %s;", n, op.s)
	if !fc.g.needsDrop(t) {
		return operand{s: n, place: true, ty: t}
	}
	flag := "d" + n
	fc.ext.decls = append(fc.ext.decls,
		fmt.Sprintf("%suint8_t %s = 0;", ind, flag),
		fmt.Sprintf("%sRG_CS_PUSH(&%s, %s, &%s);", ind, n, fc.g.dropFn(t), flag))
	fc.line("%s = 1;", flag)
	fc.ext.scope.drops = append(fc.ext.scope.drops, dropEntry{place: n, flag: flag, ty: t})
	return operand{s: n, place: true, flag: flag, movable: true, ty: t}
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
	// temporaries of the initializer die at the end of the statement unless
	// their lifetime is extended (borrowed in an extending position, or by
	// a `ref` binding of a place projected from a temporary)
	set := map[ast.NodeID]bool{}
	fc.extendingExpr(x.Init, set)
	if isPlaceExpr(x.Init) && fc.patBorrows(x.Pat) {
		fc.extendedPlace(x.Init, set)
	}
	saved := fc.ext
	fc.ext = nil
	var ext *extCtx
	mark := 0
	if len(set) > 0 {
		ext = &extCtx{set: set, scope: fc.scopes[len(fc.scopes)-1], ind: fc.ind}
		fc.ext = ext
		mark = fc.b.Len()
	}
	fc.letInit(x)
	fc.ext = saved
	if ext != nil && len(ext.decls) > 0 {
		// declare the extended temporaries before the statement: they are
		// initialized in nested C blocks (block tails, if branches) but
		// live as long as the enclosing block
		s := fc.b.String()
		fc.b.Reset()
		fc.b.WriteString(s[:mark])
		for _, d := range ext.decls {
			fc.b.WriteString(d)
			fc.b.WriteByte('\n')
		}
		fc.b.WriteString(s[mark:])
	}
}

// letInit lowers a let statement with an initializer (and a pattern other
// than `_`).
func (fc *fnGen) letInit(x *ast.LetStmt) {
	if ip, ok := x.Pat.(*ast.IdentPat); ok && ip.Sub == nil && x.Else == nil {
		if l := fc.r.Locals[ip.ID]; l != nil && !l.ByRef {
			if rep, ok := unparen(x.Init).(*ast.RepeatExpr); ok && fc.repeatInPlace(rep, l) {
				return
			}
			fc.pushScope()
			v := fc.value(x.Init)
			if !isSimple(v) {
				v = fc.spill(v, fc.aty(x.Init))
			}
			fc.popScope()
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
	fc.pushScope()
	if isPlaceExpr(x.Init) {
		op = fc.placeOp(x.Init)
		// the pattern may move out of (or borrow) the place: temporaries
		// it is projected from stay alive with the bindings
		fc.mergeScope()
	} else {
		v := fc.value(x.Init)
		p, _ := fc.materializeNoDrop(v, it)
		fc.popScope()
		op = operand{s: p, place: true, ty: it}
	}
	fc.declarePatLocals(x.Pat)
	if x.Else != nil {
		test := fc.patTest(x.Pat, op.s, it)
		fc.line("if (!(%s)) {", test)
		fc.ind++
		fc.blockValue(x.Else, nil, true)
		fc.ind--
		fc.line("}")
	}
	if isPlaceExpr(x.Init) {
		fc.bindPatPlace(x.Pat, op.s, it, op)
	} else {
		fc.bindOwnedPat(x.Pat, op.s, it, nil, true)
	}
}

// repeatInPlace lowers `let l = [e; N]` (of a Copy element, an array
// without drop glue) by filling the local itself: a temporary array and a
// copy of it would cost as much again (C compilers do not elide copies of
// large arrays).
func (fc *fnGen) repeatInPlace(rep *ast.RepeatExpr, l *typeck.Local) bool {
	t := l.Ty
	if t == nil || t.Kind != TArray || len(fc.r.Adjust[rep.ID]) > 0 || fc.aty(rep) != t ||
		!fc.g.tcx.IsCopy(t.Elem) || fc.g.needsDrop(t) || isZST(t) {
		return false
	}
	fc.pushScope()
	v := fc.spill(fc.value(rep.Elem), fc.aty(rep.Elem))
	fc.popScope()
	fc.declLocal(l)
	if cnt := fc.g.arrayLen(t); cnt > 0 {
		fc.line("for (uint64_t i = 0; i < %d; i++) %s.a[i] = %s;", cnt, fc.localName(l), v)
	}
	fc.markInit(l)
	return true
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
	li.contDepth = len(fc.scopes) - 1 // continue also drops this iteration's bindings
	fc.blockValue(x.Body, TyUnit, true)
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
	li.contDepth = len(fc.scopes) - 1 // continue also drops this iteration's bindings
	fc.condJump(x.Cond, li.brk, li.depth)
	fc.blockValue(x.Body, TyUnit, true)
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
	li.contDepth = len(fc.scopes) - 1 // continue also drops this iteration's bindings
	next := fc.callResolved(info.Next.Def, info.Next.Args, []string{"&" + it}, optTy)
	n := fc.spill(next, optTy)
	someV := fc.g.tcx.Lang("Some").Variant
	fc.line("if (!%s) goto %s;", fc.g.enumIs(optTy, n, someV), li.brk)
	item := fmt.Sprintf("%s.f0", fc.g.enumPayload(optTy, n, someV.Index))
	fc.bindOwnedPat(x.Pat, item, info.ItemTy, nil, false)
	fc.blockValue(x.Body, TyUnit, true)
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
	if li.isBlock {
		tok := fmt.Sprintf("/*@%s_%d@*/", li.brk, len(li.brks))
		fc.line("%s", tok)
		li.brks = append(li.brks, brkSite{token: tok, n: len(li.esc)})
	}
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
	v := fc.blockValue(x.Block, t, false)
	if li.result != "" && v != fc.unit() {
		fc.line("%s = %s;", li.result, v)
	}
	if len(li.brks) > 0 {
		// a break skips the creation of the tail temporaries that outlive
		// the block: give it their (inactive) cleanup entries
		src := fc.b.String()
		for _, site := range li.brks {
			var b strings.Builder
			for _, e := range li.esc[site.n:] {
				k := 0
				for j := e.target + 1; j < li.depth; j++ {
					k += csCount(fc.scopes[j].drops)
				}
				fmt.Fprintf(&b, "RG_CS_INS(%d, &%s, %s, &%s); ", k, e.place, e.drop, e.flag)
			}
			src = strings.Replace(src, site.token, b.String(), 1)
		}
		fc.b.Reset()
		fc.b.WriteString(src)
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
	// a condition (or guard, or operand of a let chain) is a terminating
	// scope: its temporaries are dropped before the branch is taken
	fc.pushScope()
	v := fc.value(c)
	if len(fc.scopes[len(fc.scopes)-1].drops) > 0 {
		v = fc.spill(v, TyBool)
	}
	fc.popScope()
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
			parts = append(parts, fmt.Sprintf("RG_CS_POP(%d);", csCount(s.drops)))
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
	elseBranch := func() {
		if x.Else == nil {
			return
		}
		var ev string
		switch e := x.Else.(type) {
		case *ast.BlockExpr:
			ev = fc.blockValue(e.Block, t, true)
		default:
			ev = fc.coerceValue(x.Else, t)
		}
		if res != "" {
			fc.line("%s = %s;", res, ev)
		}
	}
	fc.line("{")
	fc.ind++
	depth := len(fc.scopes)
	fc.pushScope()
	// Before Rust 2024 the temporaries of an `if let` scrutinee live until
	// the end of the whole if/else: the else branch runs inside that scope.
	_, isLet := unparen(x.Cond).(*ast.LetExpr)
	keepTemps := isLet && x.Else != nil && !fc.edition2024()
	if keepTemps {
		fc.condJump(x.Cond, elseL, depth+1)
	} else {
		fc.condJump(x.Cond, elseL, depth)
	}
	v := fc.blockValue(x.Then, t, true)
	if res != "" {
		fc.line("%s = %s;", res, v)
	}
	if keepTemps {
		thenEnd := fc.newName("thenend")
		fc.line("goto %s;", thenEnd)
		fc.line("%s:;", elseL)
		elseBranch()
		fc.line("%s:;", thenEnd)
		fc.popScope()
		fc.ind--
		fc.line("}")
	} else {
		fc.popScope()
		fc.line("goto %s;", endL)
		fc.ind--
		fc.line("}")
		fc.line("%s:;", elseL)
		elseBranch()
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
