package borrowck

import (
	"fmt"
	"strings"

	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// ---------------------------------------------------------------------------
// Bit sets

type bitset []uint64

func newBitset(n int) bitset { return make(bitset, (n+63)/64) }

func (s bitset) has(i int) bool { return s[i/64]&(1<<(uint(i)%64)) != 0 }
func (s bitset) set(i int)      { s[i/64] |= 1 << (uint(i) % 64) }
func (s bitset) clear(i int)    { s[i/64] &^= 1 << (uint(i) % 64) }
func (s bitset) copyFrom(o bitset) {
	copy(s, o)
}
func (s bitset) union(o bitset) bool {
	changed := false
	for i := range s {
		n := s[i] | o[i]
		if n != s[i] {
			s[i] = n
			changed = true
		}
	}
	return changed
}
func (s bitset) empty() bool {
	for _, w := range s {
		if w != 0 {
			return false
		}
	}
	return true
}

func (s bitset) clone() bitset { c := make(bitset, len(s)); copy(c, s); return c }

// ---------------------------------------------------------------------------
// Move paths (rustc's MoveData): places made of fields, Box derefs and
// constant indices.

type movePath struct {
	key      string
	local    LocalID
	parent   int
	children []int
	place    Place
}

type moveData struct {
	paths []*movePath
	byKey map[string]int
	roots map[LocalID]int
}

func projKey(pr Proj) string {
	switch pr.Kind {
	case PDeref:
		return "*"
	case PField:
		return fmt.Sprintf("f%d:%d", pr.Variant, pr.Field)
	case PIndex:
		return fmt.Sprintf("c%d", pr.Field)
	}
	return "?"
}

// movable reports whether the projection continues a move path.
func movable(pr Proj) bool {
	switch pr.Kind {
	case PDeref:
		return pr.Deref == DBox
	case PField:
		return true
	case PIndex:
		return pr.Index == -2
	}
	return false
}

// pathPrefix returns how many projections of p form its move path.
func pathPrefix(p Place) int {
	for i, pr := range p.Proj {
		if !movable(pr) {
			return i
		}
	}
	return len(p.Proj)
}

func (md *moveData) pathFor(p Place, n int) int {
	key := fmt.Sprintf("%d", p.Local)
	idx, ok := md.byKey[key]
	if !ok {
		idx = len(md.paths)
		md.paths = append(md.paths, &movePath{key: key, local: p.Local, parent: -1, place: Place{Local: p.Local}})
		md.byKey[key] = idx
		md.roots[p.Local] = idx
	}
	for i := 0; i < n; i++ {
		key += "/" + projKey(p.Proj[i])
		c, ok := md.byKey[key]
		if !ok {
			c = len(md.paths)
			md.paths = append(md.paths, &movePath{key: key, local: p.Local, parent: idx, place: Place{Local: p.Local, Proj: append([]Proj(nil), p.Proj[:i+1]...)}})
			md.byKey[key] = c
			md.paths[idx].children = append(md.paths[idx].children, c)
		}
		idx = c
	}
	return idx
}

// lookup returns the move path of p's prefix if it exists.
func (md *moveData) lookup(p Place) (int, int) {
	n := pathPrefix(p)
	key := fmt.Sprintf("%d", p.Local)
	idx, ok := md.byKey[key]
	if !ok {
		return -1, 0
	}
	for i := 0; i < n; i++ {
		key += "/" + projKey(p.Proj[i])
		c, ok := md.byKey[key]
		if !ok {
			return idx, i
		}
		idx = c
	}
	return idx, n
}

func (md *moveData) forEachDesc(i int, f func(int)) {
	f(i)
	for _, c := range md.paths[i].children {
		md.forEachDesc(c, f)
	}
}

func buildMoveData(body *Body) *moveData {
	md := &moveData{byKey: map[string]int{}, roots: map[LocalID]int{}}
	for i := range body.Locals {
		md.pathFor(Place{Local: LocalID(i)}, 0)
	}
	visit := func(p Place) { md.pathFor(p, pathPrefix(p)) }
	for _, bl := range body.Blocks {
		for _, s := range bl.Stmts {
			if s.Kind == SAssign || s.Kind == SDrop {
				visit(s.Place)
			}
			if s.Kind == SAssign {
				for _, o := range s.Rv.Ops {
					if o.Kind != OConst {
						visit(o.Place)
					}
				}
			}
		}
		if bl.Term.Kind == TCall {
			visit(bl.Term.Dest)
			for _, o := range bl.Term.Args {
				if o.Kind != OConst {
					visit(o.Place)
				}
			}
		}
	}
	return md
}

// ---------------------------------------------------------------------------
// Diagnostics

type diag struct {
	span lexer.Span
	msg  string
}

type checker struct {
	body  *Body
	md    *moveData
	diags []diag
	seen  map[string]bool
	ctx   *Ctx
}

func (c *checker) errorf(sp lexer.Span, code, format string, args ...any) {
	msg := fmt.Sprintf("%s: error[%s]: %s", sp.Lo, code, fmt.Sprintf(format, args...))
	if c.seen[msg] {
		return
	}
	c.seen[msg] = true
	c.diags = append(c.diags, diag{span: sp, msg: msg})
}

// describe renders a place for diagnostics (`x`, `x.f`, `*x`).
func (c *checker) describe(p Place) string {
	ld := c.body.Locals[p.Local]
	name := ld.Name
	if name == "" || name == "__arg" {
		name = "value"
		if ld.Kind == LTemp {
			return "value"
		}
	}
	s := name
	for _, pr := range p.Proj {
		switch pr.Kind {
		case PDeref:
			if c.body.Locals[p.Local].Kind == LUpvar && s == name {
				continue
			}
			s = "*" + s
		case PField:
			f := pr.Name
			if f == "" {
				f = fmt.Sprint(pr.Field)
			}
			if strings.HasPrefix(s, "*") {
				s = "(" + s + ")"
			}
			s += "." + f
		case PIndex:
			s += "[..]"
		}
	}
	return s
}

func isUserLocal(ld *LocalDecl) bool {
	return (ld.Kind == LVar || ld.Kind == LArg || ld.Kind == LUpvar) && ld.Name != "" && ld.Name != "__arg"
}

// ---------------------------------------------------------------------------
// Initialization / moves

type initState struct {
	uninit bitset // maybe uninitialized
	moved  bitset // maybe moved
	ever   bitset // ever initialized (since StorageLive)
	minit  bitset // maybe initialized
}

func (st *initState) clone() *initState {
	return &initState{st.uninit.clone(), st.moved.clone(), st.ever.clone(), st.minit.clone()}
}

func (st *initState) join(o *initState) bool {
	a := st.uninit.union(o.uninit)
	b := st.moved.union(o.moved)
	c := st.ever.union(o.ever)
	d := st.minit.union(o.minit)
	return a || b || c || d
}

func successors(t Term) []int { return t.Targets }

func (c *checker) checkMoves() {
	body := c.body
	md := c.md
	n := len(md.paths)
	entry := &initState{newBitset(n), newBitset(n), newBitset(n), newBitset(n)}
	for i, ld := range body.Locals {
		root := md.roots[LocalID(i)]
		if ld.Kind == LArg || ld.Kind == LUpvar {
			md.forEachDesc(root, func(j int) { entry.ever.set(j); entry.minit.set(j) })
			continue
		}
		md.forEachDesc(root, func(j int) { entry.uninit.set(j) })
	}
	in := make([]*initState, len(body.Blocks))
	in[0] = entry
	work := []int{0}
	queued := map[int]bool{0: true}
	report := false
	// drops of locals that are definitely not initialized there (moved
	// out): no-ops, which use nothing
	var deadDrops [][2]int
	run := func(bi int, st *initState) *initState {
		st = st.clone()
		bl := body.Blocks[bi]
		for si, s := range bl.Stmts {
			if report && s.Kind == SDrop && len(s.Place.Proj) == 0 {
				live := false
				md.forEachDesc(md.roots[s.Place.Local], func(j int) { live = live || st.minit.has(j) })
				if !live {
					deadDrops = append(deadDrops, [2]int{bi, si})
				}
			}
			c.stmtMoves(st, s, report)
		}
		c.termMoves(st, bl.Term, report)
		return st
	}
	for len(work) > 0 {
		bi := work[0]
		work = work[1:]
		queued[bi] = false
		out := run(bi, in[bi])
		for _, sc := range successors(body.Blocks[bi].Term) {
			if in[sc] == nil {
				in[sc] = out.clone()
			} else if !in[sc].join(out) {
				continue
			}
			if !queued[sc] {
				queued[sc] = true
				work = append(work, sc)
			}
		}
	}
	report = true
	for bi := range body.Blocks {
		if in[bi] != nil {
			run(bi, in[bi])
		}
	}
	for _, d := range deadDrops {
		body.Blocks[d[0]].Stmts[d[1]].Kind = SNop
	}
}

func (st *initState) assignPath(md *moveData, i int) {
	md.forEachDesc(i, func(j int) {
		st.uninit.clear(j)
		st.moved.clear(j)
		st.ever.set(j)
		st.minit.set(j)
	})
}

func (st *initState) movePath(md *moveData, i int) {
	md.forEachDesc(i, func(j int) {
		st.uninit.set(j)
		st.moved.set(j)
		st.minit.clear(j)
	})
}

// checkInit reports a use of place p that is not (fully) initialized.
// deep: the whole value is used (partially moved values are errors).
func (c *checker) checkInit(st *initState, p Place, sp lexer.Span, verb string, deep bool) {
	md := c.md
	idx, n := md.lookup(p)
	if idx < 0 {
		return
	}
	// a use through a deref of an uninitialized pointer reads the pointer
	full := n == pathPrefix(p)
	bad := -1
	for j := idx; j >= 0; j = md.paths[j].parent {
		if st.uninit.has(j) {
			bad = j
		}
	}
	if bad < 0 && deep && full {
		md.forEachDesc(idx, func(j int) {
			if bad < 0 && st.uninit.has(j) {
				bad = j
			}
		})
	}
	if bad < 0 {
		return
	}
	ld := c.body.Locals[p.Local]
	if !isUserLocal(ld) && ld.Kind != LTemp {
		return
	}
	if ld.Kind == LTemp {
		// compiler temporaries are always initialized before use; a
		// failure here is a limitation of the MIR model
		return
	}
	what := c.describe(p)
	if st.moved.has(bad) {
		if bad != idx && md.paths[bad].parent >= 0 && isDesc(md, bad, idx) {
			c.errorf(sp, "E0382", "%s of partially moved value: `%s`", verb, what)
			return
		}
		c.errorf(sp, "E0382", "%s of moved value: `%s`", verb, what)
		return
	}
	if bad != idx && isDesc(md, bad, idx) {
		c.errorf(sp, "E0381", "%s binding `%s` isn't fully initialized", verb, what)
		return
	}
	c.errorf(sp, "E0381", "used binding `%s` isn't initialized", what)
}

// isDesc reports whether path a is a strict descendant of path b.
func isDesc(md *moveData, a, b int) bool {
	for j := md.paths[a].parent; j >= 0; j = md.paths[j].parent {
		if j == b {
			return true
		}
	}
	return false
}

func (c *checker) useOperand(st *initState, o Operand, sp lexer.Span) {
	if o.Kind == OConst {
		return
	}
	c.checkInit(st, o.Place, sp, "use", true)
	c.checkDerefBases(st, o.Place, sp)
	if o.Kind == OMove {
		c.checkMoveOut(o.Place, sp)
		if idx, n := c.md.lookup(o.Place); idx >= 0 && n == pathPrefix(o.Place) && n == len(o.Place.Proj) {
			st.movePath(c.md, idx)
		}
	}
}

// checkDerefBases checks that the pointers dereferenced by p are
// initialized.
func (c *checker) checkDerefBases(st *initState, p Place, sp lexer.Span) {
	for i, pr := range p.Proj {
		if pr.Kind == PDeref && pr.Deref != DBox {
			c.checkInit(st, Place{Local: p.Local, Proj: p.Proj[:i]}, sp, "use", true)
			return
		}
	}
}

// checkMoveOut reports moves out of places that cannot be moved from.
func (c *checker) checkMoveOut(p Place, sp lexer.Span) {
	if !c.ctx.report {
		return
	}
	for i, pr := range p.Proj {
		switch {
		case pr.Kind == PField && pr.DropTy != nil:
			c.errorf(sp, "E0509", "cannot move out of type `%s`, which implements the `Drop` trait", pr.DropTy)
			return
		case pr.Kind == PDeref && pr.Deref != DBox:
			base := Place{Local: p.Local, Proj: p.Proj[:i]}
			if c.body.Locals[p.Local].Kind == LUpvar && i == 0 {
				c.errorf(sp, "E0507", "cannot move out of `%s`, a captured variable in an `%s` closure", c.describe(p), closureKindName(c.body.ClosureKind))
				return
			}
			kind := "a shared reference"
			switch pr.Deref {
			case DRefMut:
				kind = "a mutable reference"
			case DRaw, DRawMut:
				kind = "a raw pointer"
			}
			if c.body.Locals[base.Local].Kind == LTemp && len(base.Proj) == 0 {
				c.errorf(sp, "E0507", "cannot move out of a value which is behind %s", kind)
				return
			}
			c.errorf(sp, "E0507", "cannot move out of `%s` which is behind %s", c.describe(Place{Local: p.Local, Proj: p.Proj[:i+1]}), kind)
			return
		case pr.Kind == PIndex && pr.Index != -2:
			c.errorf(sp, "E0508", "cannot move out of type `%s`, a non-copy array", c.placeTyString(Place{Local: p.Local, Proj: p.Proj[:i]}))
			return
		}
	}
}

func closureKindName(k int) string {
	if k == 1 {
		return "FnMut"
	}
	return "Fn"
}

func (c *checker) placeTyString(p Place) string {
	if len(p.Proj) == 0 {
		return fmt.Sprint(c.body.Locals[p.Local].Ty)
	}
	return "[T; N]"
}

func (c *checker) stmtMoves(st *initState, s Stmt, report bool) {
	md := c.md
	c.ctx.report = report
	switch s.Kind {
	case SAssign:
		switch s.Rv.Kind {
		case RRef, RRawRef:
			verb := "borrow"
			c.checkInit(st, s.Rv.Place, s.Span, verb, true)
			c.checkDerefBases(st, s.Rv.Place, s.Span)
			if report && s.Rv.Mut && s.Rv.Kind == RRef {
				c.checkMutable(s.Rv.Place, s.Span, true)
			}
		default:
			for _, o := range s.Rv.Ops {
				if report {
					c.useOperand(st, o, s.Span)
				} else {
					c.useOperandSilent(st, o)
				}
			}
		}
		c.assignTo(st, s.Place, s.Span, s.Init, report)
		_ = md
	case SFakeRead:
		c.checkInit(st, s.Place, s.Span, "use", false)
	case SStorageDead:
		root := md.roots[s.Place.Local]
		md.forEachDesc(root, func(j int) {
			st.uninit.set(j)
			st.moved.clear(j)
			st.ever.clear(j)
			st.minit.clear(j)
		})
	}
}

func (c *checker) useOperandSilent(st *initState, o Operand) {
	if o.Kind != OMove {
		return
	}
	if idx, n := c.md.lookup(o.Place); idx >= 0 && n == pathPrefix(o.Place) && n == len(o.Place.Proj) {
		st.movePath(c.md, idx)
	}
}

// assignTo records an assignment to p and checks it.
func (c *checker) assignTo(st *initState, p Place, sp lexer.Span, init bool, report bool) {
	md := c.md
	idx, n := md.lookup(p)
	ld := c.body.Locals[p.Local]
	if report {
		c.checkDerefBases(st, p, sp)
		if len(p.Proj) == 0 {
			if !init && !ld.Mut && isUserLocal(ld) && ld.Kind != LUpvar && st.ever.has(md.roots[p.Local]) {
				if ld.Kind == LArg {
					c.errorf(sp, "E0384", "cannot assign to immutable argument `%s`", ld.Name)
				} else {
					c.errorf(sp, "E0384", "cannot assign twice to immutable variable `%s`", ld.Name)
				}
			}
		} else {
			c.checkMutable(p, sp, false)
			// assigning a field of an uninitialized struct
			if idx >= 0 && n == len(p.Proj) && n > 0 {
				par := md.paths[idx].parent
				for j := par; j >= 0; j = md.paths[j].parent {
					if st.uninit.has(j) && !st.moved.has(j) && isUserLocal(ld) {
						c.errorf(sp, "E0381", "partially assigned binding `%s` isn't fully initialized", c.describe(Place{Local: p.Local}))
						break
					}
				}
			}
		}
	}
	if idx >= 0 && n == pathPrefix(p) && n == len(p.Proj) {
		st.assignPath(md, idx)
	}
}

func (c *checker) termMoves(st *initState, t Term, report bool) {
	c.ctx.report = report
	if t.Kind != TCall {
		return
	}
	for _, o := range t.Args {
		if report {
			c.useOperand(st, o, t.Span)
		} else {
			c.useOperandSilent(st, o)
		}
	}
	c.assignTo(st, t.Dest, t.Span, true, false)
}

// ---------------------------------------------------------------------------
// Mutability (rustc's is_mutable)

// checkMutable reports E0596 (borrow) / E0594 (assignment) for places that
// cannot be mutated.
func (c *checker) checkMutable(p Place, sp lexer.Span, borrow bool) {
	ok, culprit, why := c.isMutable(p, false)
	if ok {
		return
	}
	code, verb := "E0594", "assign to"
	if borrow {
		code, verb = "E0596", "borrow"
	}
	what := c.describe(p)
	if borrow {
		what += "` as mutable"
		switch why {
		case "local":
			c.errorf(sp, code, "cannot %s `%s, as `%s` is not declared as mutable", verb, what, c.describe(Place{Local: culprit.Local}))
		case "ref":
			c.errorf(sp, code, "cannot %s `%s, as it is behind a `&` reference", verb, what)
		case "upvar":
			c.errorf(sp, code, "cannot %s `%s, as `%s` is a captured variable in a `Fn` closure", verb, what, c.body.Locals[p.Local].Name)
		default:
			c.errorf(sp, code, "cannot %s `%s", verb, what)
		}
		return
	}
	switch why {
	case "local":
		c.errorf(sp, code, "cannot %s `%s`, as `%s` is not declared as mutable", verb, what, c.describe(Place{Local: culprit.Local}))
	case "ref":
		c.errorf(sp, code, "cannot %s `%s`, which is behind a `&` reference", verb, what)
	case "upvar":
		c.errorf(sp, code, "cannot %s `%s`, as it is a captured variable in a `Fn` closure", verb, what)
	default:
		c.errorf(sp, code, "cannot %s `%s`", verb, what)
	}
}

// isMutable mirrors rustc: after a deref of `&mut`, immutable bindings of
// the reference itself are fine.
func (c *checker) isMutable(p Place, localOK bool) (bool, Place, string) {
	if len(p.Proj) == 0 {
		ld := c.body.Locals[p.Local]
		if ld.Mut || localOK || ld.Kind == LTemp || ld.Kind == LReturn {
			return true, p, ""
		}
		return false, p, "local"
	}
	last := p.Proj[len(p.Proj)-1]
	base := Place{Local: p.Local, Proj: p.Proj[:len(p.Proj)-1]}
	if last.Kind == PDeref {
		switch last.Deref {
		case DRef:
			if c.body.Locals[p.Local].Kind == LUpvar && len(base.Proj) == 0 && (!c.body.Locals[p.Local].UpvarByRef || c.body.ClosureKind == 0) {
				return false, p, "upvar"
			}
			return false, p, "ref"
		case DRaw:
			return false, p, "raw"
		case DRawMut:
			return true, p, ""
		case DRefMut:
			if ld := c.body.Locals[p.Local]; !localOK && ld.Kind == LUpvar && len(base.Proj) == 0 && !ld.UpvarByRef && !ld.Mut {
				// a by-value capture (reached through the FnMut
				// closure's `&mut self`) is as mutable as the binding
				return false, base, "local"
			}
			return c.isMutable(base, true)
		case DBox:
			return c.isMutable(base, localOK)
		}
	}
	return c.isMutable(base, localOK)
}

var _ = TyUnit
