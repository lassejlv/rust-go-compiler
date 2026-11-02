package typeck

import (
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// ConstValue caches an evaluated integer constant.
type ConstValue struct {
	Val int64
	OK  bool
}

// evalConstInt evaluates simple integer constant expressions (array
// lengths, enum discriminants, const items used in types).
func (t *Tcx) evalConstInt(ac *astConv, e ast.Expr) (int64, bool) {
	return t.evalConstEnv(ac, e, nil)
}

type constEnv struct {
	vars  map[string]int64
	steps int
}

type constReturn struct{ v int64 }
type constBreak struct{}
type constFail struct{}

// evalConstEnv evaluates with local variables (for const fn bodies).
func (t *Tcx) evalConstEnv(ac *astConv, e ast.Expr, env *constEnv) (v int64, ok bool) {
	if env != nil {
		env.steps++
		if env.steps > 1000000 {
			return 0, false
		}
	}
	switch x := e.(type) {
	case *ast.PathExpr:
		if env != nil && x.QSelf == nil && len(x.Path.Segments) == 1 {
			if v, ok := env.vars[x.Path.Segments[0].Ident.Name]; ok {
				return v, true
			}
		}
	case *ast.CallExpr:
		return t.evalConstCall(ac, x, env)
	case *ast.IfExpr:
		c, ok := t.evalConstEnv(ac, x.Cond, env)
		if !ok {
			return 0, false
		}
		if c != 0 {
			return t.evalConstBlock(ac, x.Then, env)
		}
		if x.Else == nil {
			return 0, true
		}
		return t.evalConstEnv(ac, x.Else, env)
	case *ast.ReturnExpr:
		if env == nil || x.Value == nil {
			return 0, false
		}
		v, ok := t.evalConstEnv(ac, x.Value, env)
		if !ok {
			return 0, false
		}
		panic(constReturn{v})
	case *ast.WhileExpr:
		if env == nil {
			return 0, false
		}
		for {
			c, ok := t.evalConstEnv(ac, x.Cond, env)
			if !ok {
				return 0, false
			}
			if c == 0 {
				return 0, true
			}
			if _, ok := t.evalConstBlock(ac, x.Body, env); !ok {
				return 0, false
			}
		}
	case *ast.AssignExpr:
		if env == nil {
			return 0, false
		}
		if p, ok := x.L.(*ast.PathExpr); ok && len(p.Path.Segments) == 1 {
			v, ok := t.evalConstEnv(ac, x.R, env)
			if !ok {
				return 0, false
			}
			env.vars[p.Path.Segments[0].Ident.Name] = v
			return 0, true
		}
		return 0, false
	case *ast.AssignOpExpr:
		if env == nil {
			return 0, false
		}
		if p, ok := x.L.(*ast.PathExpr); ok && len(p.Path.Segments) == 1 {
			v, ok := t.evalConstEnv(ac, &ast.BinaryExpr{Op: x.Op, L: x.L, R: x.R}, env)
			if !ok {
				return 0, false
			}
			env.vars[p.Path.Segments[0].Ident.Name] = v
			return 0, true
		}
		return 0, false
	}
	switch x := e.(type) {
	case *ast.LitExpr:
		switch x.Kind {
		case ast.LitInt, ast.LitChar, ast.LitByte:
			return int64(x.IntVal), true
		case ast.LitBool:
			if x.Value == "true" {
				return 1, true
			}
			return 0, true
		}
	case *ast.ParenExpr:
		return t.evalConstEnv(ac, x.X, env)
	case *ast.BlockExpr:
		if tail := blockTail(x.Block); tail != nil && len(x.Block.Stmts) == 1 {
			return t.evalConstEnv(ac, tail, env)
		}
		if env != nil {
			return t.evalConstBlock(ac, x.Block, env)
		}
	case *ast.CastExpr:
		v, ok := t.evalConstEnv(ac, x.X, env)
		if !ok {
			return 0, false
		}
		ty := ac.lowerTy(x.Ty)
		if ty.Kind == TInt {
			return truncInt(v, ty.IntKind()), true
		}
		return v, true
	case *ast.UnaryExpr:
		v, ok := t.evalConstEnv(ac, x.X, env)
		if !ok {
			return 0, false
		}
		switch x.Op {
		case ast.UnNeg:
			return -v, true
		case ast.UnNot:
			return ^v, true
		}
	case *ast.BinaryExpr:
		a, ok1 := t.evalConstEnv(ac, x.L, env)
		b, ok2 := t.evalConstEnv(ac, x.R, env)
		if !ok1 || !ok2 {
			return 0, false
		}
		switch x.Op {
		case ast.BinAdd:
			return a + b, true
		case ast.BinSub:
			return a - b, true
		case ast.BinMul:
			return a * b, true
		case ast.BinDiv:
			if b == 0 {
				return 0, false
			}
			return a / b, true
		case ast.BinRem:
			if b == 0 {
				return 0, false
			}
			return a % b, true
		case ast.BinShl:
			return a << uint(b), true
		case ast.BinShr:
			return a >> uint(b), true
		case ast.BinBitAnd:
			return a & b, true
		case ast.BinBitOr:
			return a | b, true
		case ast.BinBitXor:
			return a ^ b, true
		case ast.BinEq:
			return b2i(a == b), true
		case ast.BinNe:
			return b2i(a != b), true
		case ast.BinLt:
			return b2i(a < b), true
		case ast.BinLe:
			return b2i(a <= b), true
		case ast.BinGt:
			return b2i(a > b), true
		case ast.BinGe:
			return b2i(a >= b), true
		}
	case *ast.PathExpr:
		segs := pathSegs(x.Path)
		if len(segs) == 2 && IsPrimInt(segs[0]) {
			it, _ := IntTyByName(segs[0])
			switch segs[1] {
			case "MAX":
				return intMax(it), true
			case "MIN":
				return intMin(it), true
			case "BITS":
				return int64(it.Bits()), true
			}
		}
		if x.QSelf != nil {
			return 0, false
		}
		pr, err := t.R.ResolvePath(ac.scope, segs, x.Path.Global, ValueNS, ac.firstLookup)
		if err != nil || pr.Consumed != len(segs) || pr.Res.Kind != ResDef {
			// assoc consts on types: Self::N / Type::N
			if err == nil && pr.Consumed == len(segs)-1 && pr.Res.Kind == ResDef || (err == nil && pr.Res.Kind == ResSelfTy) {
				return t.evalAssocConst(ac, pr.Res, segs[len(segs)-1])
			}
			return 0, false
		}
		d := pr.Res.Def
		if d.Kind == DefConst && d.Const.Expr != nil {
			if cv, ok := t.constCache[d]; ok {
				return cv.Val, cv.OK
			}
			t.constCache[d] = &ConstValue{}
			t.Generics(d)
			v, ok := t.evalConstInt(t.itemConv(d), d.Const.Expr)
			t.constCache[d] = &ConstValue{Val: v, OK: ok}
			return v, ok
		}
	case *ast.CallExpr:
		// size_of::<T>() etc. are not supported in const context here
	}
	return 0, false
}

func (t *Tcx) evalAssocConst(ac *astConv, res Res, name string) (int64, bool) {
	var impls []*Def
	switch res.Kind {
	case ResSelfTy:
		if res.Def != nil && res.Def.Kind == DefImpl {
			impls = []*Def{res.Def}
		}
	case ResDef:
		if res.Def.Kind == DefStruct || res.Def.Kind == DefEnum {
			impls = t.inherent[SimplifyKey(Adt(res.Def, nil))]
		}
	}
	for _, impl := range impls {
		if it, ok := impl.Impl.Items[name]; ok && it.Kind == DefConst && it.Const.Expr != nil {
			t.Generics(it)
			return t.evalConstInt(t.itemConv(it), it.Const.Expr)
		}
	}
	return 0, false
}

func b2i(b bool) int64 {
	if b {
		return 1
	}
	return 0
}

// IsPrimInt reports whether s names an integer type.
func IsPrimInt(s string) bool {
	_, ok := IntTyByName(s)
	return ok && !strings.HasPrefix(s, "f")
}

func intMax(it IntTy) int64 {
	bits := it.Bits()
	if bits >= 64 {
		if it.Signed() {
			return 1<<63 - 1
		}
		return -1 // u64::MAX as bit pattern
	}
	if it.Signed() {
		return 1<<(bits-1) - 1
	}
	return 1<<bits - 1
}

func intMin(it IntTy) int64 {
	if !it.Signed() {
		return 0
	}
	if it.Bits() >= 64 {
		return -1 << 63
	}
	return -1 << (it.Bits() - 1)
}

func truncInt(v int64, it IntTy) int64 {
	bits := it.Bits()
	if bits >= 64 {
		return v
	}
	mask := int64(1)<<bits - 1
	v &= mask
	if it.Signed() && v&(int64(1)<<(bits-1)) != 0 {
		v -= int64(1) << bits
	}
	return v
}

func (t *Tcx) evalConstBlock(ac *astConv, b *ast.Block, env *constEnv) (int64, bool) {
	var last int64
	for i, st := range b.Stmts {
		switch x := st.(type) {
		case *ast.LetStmt:
			ip, ok := x.Pat.(*ast.IdentPat)
			if !ok || x.Init == nil {
				return 0, false
			}
			v, ok := t.evalConstEnv(ac, x.Init, env)
			if !ok {
				return 0, false
			}
			env.vars[ip.Name.Name] = v
		case *ast.ExprStmt:
			v, ok := t.evalConstEnv(ac, x.Expr, env)
			if !ok {
				return 0, false
			}
			if i == len(b.Stmts)-1 && !x.Semi {
				last = v
			}
		default:
			return 0, false
		}
	}
	return last, true
}

// evalConstCall evaluates a call to a const fn with integer arguments.
func (t *Tcx) evalConstCall(ac *astConv, x *ast.CallExpr, env *constEnv) (res int64, ok bool) {
	pe, isPath := x.Fn.(*ast.PathExpr)
	if !isPath || pe.QSelf != nil {
		return 0, false
	}
	pr, err := t.R.ResolvePath(ac.scope, pathSegs(pe.Path), pe.Path.Global, ValueNS, ac.firstLookup)
	if err != nil || pr.Consumed != len(pe.Path.Segments) || pr.Res.Kind != ResDef || pr.Res.Def.Kind != DefFn {
		return 0, false
	}
	d := pr.Res.Def
	fa := d.Fn.Ast
	if fa.Body == nil || len(fa.Decl.Params) != len(x.Args) {
		return 0, false
	}
	nenv := &constEnv{vars: map[string]int64{}}
	if env != nil {
		nenv.steps = env.steps
	}
	for i, p := range fa.Decl.Params {
		ip, ok := p.Pat.(*ast.IdentPat)
		if !ok {
			return 0, false
		}
		v, ok := t.evalConstEnv(ac, x.Args[i], env)
		if !ok {
			return 0, false
		}
		nenv.vars[ip.Name.Name] = v
	}
	t.Generics(d)
	fac := t.itemConv(d)
	defer func() {
		if r := recover(); r != nil {
			if cr, isRet := r.(constReturn); isRet {
				res, ok = cr.v, true
				return
			}
			panic(r)
		}
	}()
	return t.evalConstBlock(fac, fa.Body, nenv)
}
