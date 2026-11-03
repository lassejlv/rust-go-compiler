package expand

import (
	"errors"
	"sort"

	"rustgo/compiler/ast"
)

// Saving and restoring the expander state a crate leaves behind (for the
// standard library cache, see driver): its macro_rules! definitions, the
// renaming imports of macros and macro imports, and the hygiene mark
// counter.

// UsedExternalInput reports whether an expansion so far read the process
// environment or a file (env!, include!, ...) or could have run a
// procedural macro: its result then depends on more than the crate's
// source.
func (e *Expander) UsedExternalInput() bool { return e.external || len(e.ProcMacros) > 0 }

// HygieneMarks returns the number of hygiene marks allocated so far;
// SetHygieneMarks sets it (after decoding a crate expanded elsewhere).
func HygieneMarks() uint32     { return hygieneMarks }
func SetHygieneMarks(n uint32) { hygieneMarks = n }

// EncodeCrateState writes the state crate left in e.
func (e *Expander) EncodeCrateState(crate string, enc *ast.Encoder) {
	if _, ok := e.macros[crate]; !ok {
		enc.Fail(errors.New("expand: crate not expanded"))
		return
	}
	defs := e.macros[crate]
	enc.Uint(uint64(len(defs)))
	for _, k := range sortedMapKeys(defs) {
		d := defs[k]
		enc.String(k)
		enc.String(d.name)
		enc.String(d.crate)
		enc.Bool(d.exported)
		enc.Span(d.span)
		enc.TokenTrees(d.body)
	}
	al := e.aliases[crate]
	enc.Uint(uint64(len(al)))
	for _, k := range sortedMapKeys(al) {
		enc.String(k)
		enc.String(al[k])
	}
	mi := e.macroImports[crate]
	enc.Uint(uint64(len(mi)))
	for _, k := range sortedMapKeys(mi) {
		enc.String(k)
		enc.String(mi[k].crate)
		enc.String(mi[k].name)
		enc.Bool(mi[k].proc)
	}
}

// DecodeCrateState decodes the state of crate written by
// EncodeCrateState; commit installs it in e, as if the crate had just been
// expanded by e.
func (e *Expander) DecodeCrateState(crate string, dec *ast.Decoder) (commit func(), err error) {
	if _, ok := e.macros[crate]; ok {
		return nil, errors.New("expand: crate already expanded")
	}
	n, err := dec.Uint()
	if err != nil {
		return nil, err
	}
	defs := map[string]*macroDef{}
	for i := uint64(0); i < n; i++ {
		var k, name, dcrate string
		var exported bool
		var sp ast.Span
		var body []ast.TokenTree
		for _, err := range []error{
			get(&k, dec.String), get(&name, dec.String), get(&dcrate, dec.String),
			get(&exported, dec.Bool), get(&sp, dec.Span), get(&body, dec.TokenTrees),
		} {
			if err != nil {
				return nil, err
			}
		}
		d, err := parseMacroRules(name, body, dcrate, sp)
		if err != nil {
			return nil, err
		}
		d.exported = exported
		defs[k] = d
	}
	var al map[string]string
	if n, err = dec.Uint(); err != nil {
		return nil, err
	}
	if n > 0 {
		al = map[string]string{}
	}
	for i := uint64(0); i < n; i++ {
		var k, v string
		if err := firstErr(get(&k, dec.String), get(&v, dec.String)); err != nil {
			return nil, err
		}
		al[k] = v
	}
	var mi map[string]macroRef
	if n, err = dec.Uint(); err != nil {
		return nil, err
	}
	if n > 0 {
		mi = map[string]macroRef{}
	}
	for i := uint64(0); i < n; i++ {
		var k string
		var r macroRef
		if err := firstErr(get(&k, dec.String), get(&r.crate, dec.String), get(&r.name, dec.String), get(&r.proc, dec.Bool)); err != nil {
			return nil, err
		}
		mi[k] = r
	}
	return func() {
		e.macros[crate] = defs
		e.crateOrder = append(e.crateOrder, crate)
		if al != nil {
			e.aliases[crate] = al
		}
		if mi != nil {
			e.macroImports[crate] = mi
		}
	}, nil
}

func get[T any](p *T, f func() (T, error)) error {
	v, err := f()
	*p = v
	return err
}

func firstErr(errs ...error) error {
	for _, err := range errs {
		if err != nil {
			return err
		}
	}
	return nil
}

func sortedMapKeys[V any](m map[string]V) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}
