package codegen

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (g *Gen) genIntrinsicFn(name string, d *Def, args []Ty) {
	// An intrinsic referenced as a function value: emit a wrapper.
	ins, out := g.tcx.InstantiateFnSig(d, args)
	var ps, vs []string
	for i, in := range ins {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
		vs = append(vs, fmt.Sprintf("a%d", i))
	}
	fg := newFnGen(g, nil)
	body := fg.intrinsic(d, args, vs, out, d.Span)
	sig := fmt.Sprintf("static %s %s(%s)", g.ctype(out), name, joinParams(ps))
	fmt.Fprintf(&g.protos, "%s;\n", sig)
	fmt.Fprintf(&g.fns, "%s {\n%s  return %s;\n}\n", sig, fg.b.String(), body)
}

func isUnsignedTy(t Ty) bool { return t.Kind == TInt && !t.IsSigned() }

// unsignedOf returns the unsigned C type of the same width.
func (fc *fnGen) unsignedC(t Ty) string {
	switch t.IntKind() {
	case I8, U8:
		return "uint8_t"
	case I16, U16:
		return "uint16_t"
	case I32, U32:
		return "uint32_t"
	case I64, U64, Isize, Usize:
		return "uint64_t"
	}
	return "unsigned __int128"
}

// intrinsic lowers a call to a compiler intrinsic.
func (fc *fnGen) intrinsic(d *Def, args []Ty, v []string, ret Ty, sp ast.Span) string {
	g := fc.g
	ct := func(t Ty) string { g.defineByValue(t); return g.ctype(t) }
	name := d.Intrinsic
	if s, ok := coroIntrinsic(name, v); ok {
		return s
	}
	arg := func(i int) Ty { return args[i] }
	switch name {
	case "size_of":
		if isZST(arg(0)) {
			return "((uint64_t)0)"
		}
		return fmt.Sprintf("((uint64_t)sizeof(%s))", ct(arg(0)))
	case "align_of":
		if isZST(arg(0)) {
			return "((uint64_t)1)"
		}
		return fmt.Sprintf("((uint64_t)_Alignof(%s))", ct(arg(0)))
	case "size_of_val", "align_of_val":
		t := arg(0)
		isSize := name == "size_of_val"
		switch t.Kind {
		case TStr:
			if isSize {
				return fmt.Sprintf("((uint64_t)(%s).meta)", v[0])
			}
			return "((uint64_t)1)"
		case TSlice:
			if isSize {
				return fmt.Sprintf("((uint64_t)((%s).meta * sizeof(%s)))", v[0], ct(t.Elem))
			}
			return fmt.Sprintf("((uint64_t)_Alignof(%s))", ct(t.Elem))
		case TDyn:
			if isSize {
				return fmt.Sprintf("((uint64_t)(uintptr_t)((void**)(%s).meta)[1])", v[0])
			}
			return fmt.Sprintf("((uint64_t)(uintptr_t)((void**)(%s).meta)[2])", v[0])
		case TAdt:
			if t.IsUnsized() {
				if isSize {
					return g.unsizedSize(t, "("+v[0]+").meta")
				}
				return g.unsizedAlign(t, "("+v[0]+").meta")
			}
		}
		if isZST(t) {
			if isSize {
				return "((uint64_t)0)"
			}
			return "((uint64_t)1)"
		}
		if isSize {
			return fmt.Sprintf("((uint64_t)sizeof(%s))", ct(t))
		}
		return fmt.Sprintf("((uint64_t)_Alignof(%s))", ct(t))
	case "needs_drop":
		if g.needsDrop(arg(0)) {
			return "1"
		}
		return "0"
	case "type_name":
		s := rustcTypeName(arg(0))
		return fmt.Sprintf("((Fat){ (void*)%s, %d })", cString(s), len(s))
	case "alloc":
		return fmt.Sprintf("rg_alloc(%s, %s)", v[0], v[1])
	case "realloc":
		return fmt.Sprintf("rg_realloc(%s, %s, %s, %s)", v[0], v[1], v[2], v[3])
	case "dealloc":
		return fmt.Sprintf("(rg_dealloc(%s, %s, %s), (Unit){})", v[0], v[1], v[2])
	case "copy":
		if isZST(arg(0)) {
			return "((Unit){})"
		}
		return fmt.Sprintf("(memmove(%s, %s, (%s) * sizeof(%s)), (Unit){})", v[1], v[0], v[2], ct(arg(0)))
	case "copy_nonoverlapping":
		if isZST(arg(0)) {
			return "((Unit){})"
		}
		return fmt.Sprintf("(memcpy(%s, %s, (%s) * sizeof(%s)), (Unit){})", v[1], v[0], v[2], ct(arg(0)))
	case "write_bytes":
		if isZST(arg(0)) {
			return "((Unit){})"
		}
		return fmt.Sprintf("(memset(%s, %s, (%s) * sizeof(%s)), (Unit){})", v[0], v[1], v[2], ct(arg(0)))
	case "compare_bytes":
		return fmt.Sprintf("((int32_t)rg_memcmp(%s, %s, %s))", v[0], v[1], v[2])
	case "read":
		if isZST(arg(0)) {
			return fmt.Sprintf("((%s){})", ct(arg(0)))
		}
		return fmt.Sprintf("(*(%s*)(%s))", ct(arg(0)), v[0])
	case "write":
		if isZST(arg(0)) {
			return "((Unit){})"
		}
		return fmt.Sprintf("(*(%s*)(%s) = %s, (Unit){})", ct(arg(0)), v[0], v[1])
	case "drop_in_place":
		t := arg(0)
		if !g.needsDrop(t) {
			return "((Unit){})"
		}
		return fmt.Sprintf("(%s(%s), (Unit){})", g.dropFn(t), v[0])
	case "forget":
		return "((Unit){})"
	case "transmute":
		from, to := arg(0), arg(1)
		if isZST(to) {
			return fmt.Sprintf("((%s){})", ct(to))
		}
		tmp := fc.spill(v[0], from)
		return fmt.Sprintf("(*(%s*)&%s)", ct(to), tmp)
	case "uninit", "zeroed":
		return fmt.Sprintf("((%s){})", ct(arg(0)))
	case "ptr_add", "ptr_add_mut":
		if isZST(arg(0)) {
			return v[0]
		}
		return fmt.Sprintf("(%s + %s)", v[0], v[1])
	case "ptr_offset", "ptr_offset_mut":
		if isZST(arg(0)) {
			return v[0]
		}
		return fmt.Sprintf("(%s + (int64_t)(%s))", v[0], v[1])
	case "ptr_diff":
		if isZST(arg(0)) {
			return "((int64_t)0)"
		}
		return fmt.Sprintf("((int64_t)(%s - %s))", v[0], v[1])
	case "slice_from_raw_parts", "slice_from_raw_parts_mut", "str_from_raw_parts":
		return fmt.Sprintf("((Fat){ (void*)(%s), (uint64_t)(%s) })", v[0], v[1])
	case "slice_len", "str_len":
		return fmt.Sprintf("((uint64_t)(%s).meta)", v[0])
	case "slice_ptr":
		return fmt.Sprintf("((%s*)(%s).ptr)", ct(arg(0)), v[0])
	case "str_ptr":
		return fmt.Sprintf("((uint8_t*)(%s).ptr)", v[0])
	case "discriminant_value":
		t := arg(0)
		if t.Kind == TAdt && t.Def.Adt.Kind == AdtEnum {
			return fmt.Sprintf("((int64_t)%s)", g.enumTag(t, "(*"+v[0]+")"))
		}
		return "((int64_t)0)"
	case "abort":
		return "(rg_abort(), (Unit){})"
	case "exit":
		return fmt.Sprintf("(rg_exit(%s), (Unit){})", v[0])
	case "write_stdout":
		return fmt.Sprintf("(rg_write_stdout(%s, %s), (Unit){})", v[0], v[1])
	case "write_stderr":
		return fmt.Sprintf("(rg_write_stderr(%s, %s), (Unit){})", v[0], v[1])
	case "flush_stdout":
		return "(rg_flush_stdout(), (Unit){})"
	case "panic_impl":
		return fmt.Sprintf("(rg_panic_impl(%s, %s, %s, %s), (Unit){})", v[0], v[1], v[2], v[3])
	case "wrapping_add", "wrapping_sub", "wrapping_mul":
		op := map[string]string{"wrapping_add": "+", "wrapping_sub": "-", "wrapping_mul": "*"}[name]
		t := arg(0)
		u := fc.unsignedC(t)
		return fmt.Sprintf("((%s)((%s)(%s) %s (%s)(%s)))", ct(t), u, v[0], op, u, v[1])
	case "add_with_overflow", "sub_with_overflow", "mul_with_overflow":
		op := strings.TrimSuffix(name, "_with_overflow")
		tt := ret
		tmp := fc.newName("ov")
		fc.line("%s %s;", ct(tt), tmp)
		fc.line("%s.f1 = __builtin_%s_overflow(%s, %s, &%s.f0);", tmp, op, v[0], v[1], tmp)
		return tmp
	case "unchecked_div":
		return fmt.Sprintf("((%s)(%s / %s))", ct(arg(0)), v[0], v[1])
	case "unchecked_rem":
		return fmt.Sprintf("((%s)(%s %% %s))", ct(arg(0)), v[0], v[1])
	case "unchecked_shl":
		return fmt.Sprintf("((%s)((%s)(%s) << (%s)))", ct(arg(0)), fc.unsignedC(arg(0)), v[0], v[1])
	case "unchecked_shr":
		return fmt.Sprintf("((%s)((%s) >> (%s)))", ct(arg(0)), v[0], v[1])
	case "rotate_left", "rotate_right":
		t := arg(0)
		bits := t.IntKind().Bits()
		u := fc.unsignedC(t)
		x := fc.spill(fmt.Sprintf("(%s)(%s)", u, v[0]), unsignedTy(t))
		n := fc.spill(fmt.Sprintf("((uint32_t)(%s) %% %d)", v[1], bits), TyU32)
		if name == "rotate_left" {
			return fmt.Sprintf("((%s)(%s == 0 ? %s : (%s)((%s << %s) | (%s >> (%d - %s)))))", ct(t), n, x, u, x, n, x, bits, n)
		}
		return fmt.Sprintf("((%s)(%s == 0 ? %s : (%s)((%s >> %s) | (%s << (%d - %s)))))", ct(t), n, x, u, x, n, x, bits, n)
	case "ctpop":
		t := arg(0)
		if t.IntKind().Bits() == 128 {
			return fmt.Sprintf("((uint32_t)(__builtin_popcountll((uint64_t)(%s)) + __builtin_popcountll((uint64_t)((unsigned __int128)(%s) >> 64))))", v[0], v[0])
		}
		return fmt.Sprintf("((uint32_t)__builtin_popcountll((uint64_t)(%s)(%s)))", fc.unsignedC(t), v[0])
	case "ctlz":
		t := arg(0)
		bits := t.IntKind().Bits()
		x := fc.spill(fmt.Sprintf("(%s)(%s)", fc.unsignedC(t), v[0]), unsignedTy(t))
		if bits == 128 {
			return fmt.Sprintf("rg_clz128(%s)", x)
		}
		return fmt.Sprintf("((uint32_t)(%s == 0 ? %d : __builtin_clzll((uint64_t)%s) - %d))", x, bits, x, 64-bits)
	case "cttz":
		t := arg(0)
		bits := t.IntKind().Bits()
		x := fc.spill(fmt.Sprintf("(%s)(%s)", fc.unsignedC(t), v[0]), unsignedTy(t))
		if bits == 128 {
			return fmt.Sprintf("rg_ctz128(%s)", x)
		}
		return fmt.Sprintf("((uint32_t)(%s == 0 ? %d : __builtin_ctzll((uint64_t)%s)))", x, bits, x)
	case "bswap":
		t := arg(0)
		switch t.IntKind().Bits() {
		case 8:
			return v[0]
		case 16:
			return fmt.Sprintf("((%s)__builtin_bswap16((uint16_t)(%s)))", ct(t), v[0])
		case 32:
			return fmt.Sprintf("((%s)__builtin_bswap32((uint32_t)(%s)))", ct(t), v[0])
		case 64:
			return fmt.Sprintf("((%s)__builtin_bswap64((uint64_t)(%s)))", ct(t), v[0])
		}
		return fmt.Sprintf("((%s)rg_bswap128(%s))", ct(t), v[0])
	// ---- panics / unwinding
	case "panic_impl_static":
		return fmt.Sprintf("(rg_panic_impl_static(%s, %s, %s, %s), (Unit){})", v[0], v[1], v[2], v[3])
	case "panic_resume":
		return fmt.Sprintf("(rg_panic_resume(%s, %s), (Unit){})", v[0], v[1])
	case "try_call":
		return fmt.Sprintf("rg_try((Unit (*)(uint8_t*))(%s), %s)", v[0], v[1])
	case "panic_message":
		return "rg_panic_msg()"
	case "panic_message_is_static":
		return "((uint8_t)rg_pmsg_static)"
	case "thread_panicking":
		return "((uint8_t)(rg_panicking != 0))"
	// ---- threads
	case "thread_spawn":
		return fmt.Sprintf("rg_thread_spawn((Unit (*)(uint8_t*))(%s), %s, %s)", v[0], v[1], v[2])
	case "thread_join":
		return fmt.Sprintf("(rg_thread_join(%s), (Unit){})", v[0])
	case "thread_is_finished":
		return fmt.Sprintf("rg_thread_is_finished(%s)", v[0])
	case "thread_retain":
		return fmt.Sprintf("(rg_thread_retain(%s), (Unit){})", v[0])
	case "thread_release":
		return fmt.Sprintf("(rg_thread_release(%s), (Unit){})", v[0])
	case "thread_set_name":
		return fmt.Sprintf("(rg_thread_set_name(%s, %s), (Unit){})", v[0], v[1])
	case "thread_sleep_ns":
		return fmt.Sprintf("(rg_sleep_ns(%s), (Unit){})", v[0])
	case "thread_yield":
		return "(sched_yield(), (Unit){})"
	case "thread_is_main":
		return "((uint8_t)rg_is_main)"
	case "available_parallelism":
		return "rg_available_parallelism()"
	case "futex_wait":
		return fmt.Sprintf("rg_futex_wait((uint32_t*)(%s), %s, %s)", v[0], v[1], v[2])
	case "futex_wake":
		return fmt.Sprintf("rg_futex_wake((uint32_t*)(%s))", v[0])
	case "futex_wake_all":
		return fmt.Sprintf("(rg_futex_wake_all((uint32_t*)(%s)), (Unit){})", v[0])
	// ---- thread-local storage
	case "tls_new_key":
		return "rg_tls_new_key()"
	case "tls_get":
		return fmt.Sprintf("rg_tls_get(%s)", v[0])
	case "tls_set":
		return fmt.Sprintf("(rg_tls_set(%s, %s, (Unit (*)(uint8_t*))(%s)), (Unit){})", v[0], v[1], v[2])
	// ---- atomics (all sequentially consistent)
	case "atomic_load":
		return fmt.Sprintf("((%s)__atomic_load_n(%s, __ATOMIC_SEQ_CST))", ct(arg(0)), v[0])
	case "atomic_store":
		return fmt.Sprintf("(__atomic_store_n(%s, %s, __ATOMIC_SEQ_CST), (Unit){})", v[0], v[1])
	case "atomic_xchg":
		return fmt.Sprintf("((%s)__atomic_exchange_n(%s, %s, __ATOMIC_SEQ_CST))", ct(arg(0)), v[0], v[1])
	case "atomic_cxchg", "atomic_cxchg_weak":
		weak := "0"
		if name == "atomic_cxchg_weak" {
			weak = "1"
		}
		tmp := fc.newName("cx")
		exp := fc.newName("ce")
		fc.line("%s %s;", ct(ret), tmp)
		fc.line("%s %s = %s;", ct(arg(0)), exp, v[1])
		fc.line("%s.f1 = __atomic_compare_exchange_n(%s, &%s, %s, %s, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);", tmp, v[0], exp, v[2], weak)
		fc.line("%s.f0 = %s;", tmp, exp)
		return tmp
	case "atomic_xadd", "atomic_xsub", "atomic_and", "atomic_or", "atomic_xor", "atomic_nand":
		op := map[string]string{"atomic_xadd": "add", "atomic_xsub": "sub", "atomic_and": "and", "atomic_or": "or", "atomic_xor": "xor", "atomic_nand": "nand"}[name]
		return fmt.Sprintf("((%s)__atomic_fetch_%s(%s, %s, __ATOMIC_SEQ_CST))", ct(arg(0)), op, v[0], v[1])
	case "atomic_max", "atomic_min":
		return fmt.Sprintf("((%s)RG_ATOMIC_%s(%s, %s))", ct(arg(0)), strings.ToUpper(strings.TrimPrefix(name, "atomic_")), v[0], v[1])
	case "atomic_fence":
		return "(__atomic_thread_fence(__ATOMIC_SEQ_CST), (Unit){})"
	// ---- time
	case "time_monotonic_ns":
		return "rg_clock_ns(CLOCK_MONOTONIC)"
	case "time_realtime_ns":
		return "rg_clock_ns(CLOCK_REALTIME)"
	// ---- process environment
	case "args_count":
		return "((uint64_t)rg_argc)"
	case "arg_ptr":
		return fmt.Sprintf("((uint8_t*)rg_arg_ptr(%s))", v[0])
	case "arg_len":
		return fmt.Sprintf("rg_arg_len(%s)", v[0])
	case "env_get":
		return fmt.Sprintf("((uint8_t*)rg_env_get(%s, %s))", v[0], v[1])
	case "env_count":
		return "rg_env_count()"
	case "env_entry":
		return fmt.Sprintf("((uint8_t*)rg_env_entry(%s))", v[0])
	case "cstr_len":
		return fmt.Sprintf("((uint64_t)strlen((const char*)(%s)))", v[0])
	case "process_id":
		return "((uint32_t)getpid())"
	case "stdin_fill":
		return "rg_stdin_fill()"
	case "stdin_ptr":
		return "((uint8_t*)rg_stdin_ptr())"
	case "stdin_consume":
		return fmt.Sprintf("(rg_stdin_consume(%s), (Unit){})", v[0])
	case "fmt_f64":
		return fmt.Sprintf("rg_fmt_f64(%s, %s, %s, %s, %s)", v[0], v[1], v[2], v[3], v[4])
	case "parse_f64":
		return fmt.Sprintf("rg_parse_f64(%s, %s, %s)", v[0], v[1], v[2])
	}
	// float intrinsics: <op>f64 / <op>f32
	if strings.HasSuffix(name, "f64") || strings.HasSuffix(name, "f32") {
		base := name[:len(name)-3]
		f32 := strings.HasSuffix(name, "f32")
		cfn, ok := floatFns[base]
		if ok {
			if f32 {
				cfn += "f"
			}
			return fmt.Sprintf("%s(%s)", cfn, strings.Join(v, ", "))
		}
	}
	fc.g.fatalf("%s: unknown intrinsic `%s`", sp.Lo, name)
	return ""
}

func unsignedTy(t Ty) Ty {
	switch t.IntKind() {
	case I8, U8:
		return IntT(U8)
	case I16, U16:
		return IntT(U16)
	case I32, U32:
		return IntT(U32)
	case I64, U64, Isize, Usize:
		return IntT(U64)
	}
	return IntT(U128)
}

var floatFns = map[string]string{
	"sqrt": "sqrt", "powi": "rg_powi", "powf": "pow", "exp": "exp", "exp2": "exp2", "log": "log", "log2": "log2",
	"log10": "log10", "sin": "sin", "cos": "cos", "tan": "tan", "asin": "asin", "acos": "acos", "atan": "atan",
	"atan2": "atan2", "sinh": "sinh", "cosh": "cosh", "tanh": "tanh", "floor": "floor", "ceil": "ceil",
	"round": "round", "trunc": "trunc", "fabs": "fabs", "copysign": "copysign", "fma": "fma", "cbrt": "cbrt",
	"hypot": "hypot", "asinh": "asinh", "acosh": "acosh", "atanh": "atanh", "expm1": "expm1", "ln1p": "log1p",
}
