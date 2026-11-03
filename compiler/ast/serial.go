package ast

import (
	"encoding/binary"
	"errors"
	"fmt"

	"rustgo/compiler/lexer"
)

//go:generate go run gen_serial.go

// Binary serialization of (expanded) crates, used to cache the standard
// library between compiler runs. The node encoders/decoders are generated
// from the type declarations in ast.go (gen_serial.go).
//
// Node IDs, source files and hygiene marks are serialized relative to the
// values current when the crate was produced (IDBase, FileBase, HygBase)
// and rebased on decoding, so a decoded crate is what parsing and
// expanding it in the decoder's state would have produced. Encoding fails
// on anything that would not round-trip (a node reachable twice, an ID,
// file or mark from before the base).

// Encoder serializes AST values into a byte buffer.
type Encoder struct {
	IDBase   NodeID // IDs allocated after this one are serialized
	FileBase int    // spans in files with an index >= FileBase
	HygBase  uint32 // hygiene marks > HygBase

	buf  []byte
	strs map[string]uint32
	list []string
	seen map[any]struct{}
	err  error
	last lexer.Pos // start of the previous span
}

// NewEncoder returns an encoder for values created after the given state.
func NewEncoder(idBase NodeID, fileBase int, hygBase uint32) *Encoder {
	return &Encoder{IDBase: idBase, FileBase: fileBase, HygBase: hygBase,
		strs: map[string]uint32{"": 0}, list: []string{""}, seen: map[any]struct{}{}}
}

type encodeError struct{ err error }

func (e *Encoder) fail(format string, args ...any) {
	panic(encodeError{fmt.Errorf("ast encode: "+format, args...)})
}

// guard runs f, turning encoding failures (and unexpected panics, such
// as nil nodes in interfaces) into e.err.
func (e *Encoder) guard(f func()) {
	if e.err != nil {
		return
	}
	defer func() {
		if r := recover(); r != nil {
			if ee, ok := r.(encodeError); ok {
				e.err = ee.err
			} else {
				e.err = fmt.Errorf("ast encode: %v", r)
			}
		}
	}()
	f()
}

func (e *Encoder) u(x uint64) { e.buf = binary.AppendUvarint(e.buf, x) }
func (e *Encoder) i(x int64)  { e.buf = binary.AppendVarint(e.buf, x) }
func (e *Encoder) b(x bool) {
	if x {
		e.buf = append(e.buf, 1)
	} else {
		e.buf = append(e.buf, 0)
	}
}

func (e *Encoder) s(x string) {
	n, ok := e.strs[x]
	if !ok {
		n = uint32(len(e.list))
		e.strs[x] = n
		e.list = append(e.list, x)
	}
	e.u(uint64(n))
}

func (e *Encoder) mark(p any) {
	if _, ok := e.seen[p]; ok {
		e.fail("node %T reachable twice", p)
	}
	e.seen[p] = struct{}{}
}

func (e *Encoder) id(x NodeID) {
	if x == 0 {
		e.u(0)
		return
	}
	if x <= e.IDBase {
		e.fail("node ID %d before base %d", x, e.IDBase)
	}
	e.u(uint64(x - e.IDBase))
}

func (e *Encoder) hyg(h uint32) {
	if h == 0 {
		e.u(0)
		return
	}
	if h <= e.HygBase {
		e.fail("hygiene mark %d before base %d", h, e.HygBase)
	}
	e.u(uint64(h - e.HygBase))
}

// Positions are encoded relative to the previous span's start when in the
// same file (spans mostly follow each other in source order); the first
// number is 0 for the zero Pos, 1 for a position in the previous one's
// file, and 2 + the (rebased) file index otherwise.
func (e *Encoder) pos(p lexer.Pos) {
	if p == (lexer.Pos{}) {
		e.u(0)
		return
	}
	if int(p.File) < e.FileBase || p.Off < 0 || p.Line < 0 || p.Col < 0 {
		e.fail("position %+v not serializable (file base %d)", p, e.FileBase)
	}
	if p.File == e.last.File && e.last != (lexer.Pos{}) {
		e.u(1)
		e.i(int64(p.Off - e.last.Off))
		e.i(int64(p.Line - e.last.Line))
		e.u(uint64(p.Col))
	} else {
		e.u(uint64(int(p.File)-e.FileBase) + 2)
		e.u(uint64(p.Off))
		e.u(uint64(p.Line))
		e.u(uint64(p.Col))
	}
	e.last = p
}

// span encodes Hi relative to Lo.
func (e *Encoder) span(s Span) {
	e.pos(s.Lo)
	switch {
	case s.Hi == s.Lo:
		e.u(0)
	case s.Hi.File == s.Lo.File && s.Lo != (lexer.Pos{}):
		e.u(1)
		e.i(int64(s.Hi.Off - s.Lo.Off))
		e.i(int64(s.Hi.Line - s.Lo.Line))
		e.u(uint64(s.Hi.Col))
	default:
		e.u(2)
		last := e.last
		e.pos(s.Hi)
		e.last = last
	}
}

func (e *Encoder) tok(t *Token) {
	e.s(t.Text)
	e.s(t.Suffix)
	e.span(t.Span)
	e.hyg(t.Hyg)
	e.u(uint64(t.Kind))
	var fl byte
	if t.Raw {
		fl |= 1
	}
	if t.Inner {
		fl |= 2
	}
	if t.JointNext {
		fl |= 4
	}
	e.buf = append(e.buf, fl)
}

// Crate encodes a crate (except its LoadMod function).
func (e *Encoder) Crate(c *Crate) {
	e.guard(func() {
		e.encSl_Pt_Attr(c.Attrs)
		e.encSl_Pt_Item(c.Items)
		e.span(c.Span)
		e.s(c.Dir)
	})
}

// TokenTrees encodes token trees.
func (e *Encoder) TokenTrees(tts []TokenTree) { e.guard(func() { e.encSl_TokenTree(tts) }) }

// String encodes a string.
func (e *Encoder) String(s string) { e.guard(func() { e.s(s) }) }

// Uint encodes an unsigned integer.
func (e *Encoder) Uint(x uint64) { e.guard(func() { e.u(x) }) }

// Bool encodes a boolean.
func (e *Encoder) Bool(x bool) { e.guard(func() { e.b(x) }) }

// Span encodes a span.
func (e *Encoder) Span(s Span) { e.guard(func() { e.span(s) }) }

// Fail makes the encoding fail (the error is returned by Bytes).
func (e *Encoder) Fail(err error) {
	if e.err == nil {
		e.err = err
	}
}

// Bytes returns the encoding: a header, the values in encoding order and
// the string table.
func (e *Encoder) Bytes() ([]byte, error) {
	if e.err != nil {
		return nil, e.err
	}
	var out []byte
	out = binary.LittleEndian.AppendUint64(out, uint64(len(e.buf)))
	out = append(out, e.buf...)
	out = binary.AppendUvarint(out, uint64(len(e.list)))
	for _, s := range e.list {
		out = binary.AppendUvarint(out, uint64(len(s)))
	}
	for _, s := range e.list {
		out = append(out, s...)
	}
	return out, nil
}

var errCorrupt = errors.New("ast decode: corrupt data")

// Decoder deserializes values written by an Encoder, rebasing node IDs,
// files and hygiene marks.
type Decoder struct {
	IDBase   NodeID
	FileBase int
	HygBase  uint32

	data []byte
	p    int
	end  int
	strs []string
	last lexer.Pos
	decAlloc
}

// Decoded nodes and slices are carved out of larger allocations (a
// decoded crate lives as long as the compilation anyway).
const (
	slabSize  = 128
	chunkSize = 1024
)

// NewDecoder returns a decoder of data (from Encoder.Bytes) that rebases
// what it decodes onto the given state.
func NewDecoder(data []byte, idBase NodeID, fileBase int, hygBase uint32) (d *Decoder, err error) {
	defer func() {
		if r := recover(); r != nil {
			d, err = nil, errCorrupt
		}
	}()
	if len(data) < 8 {
		return nil, errCorrupt
	}
	n := binary.LittleEndian.Uint64(data)
	if n > uint64(len(data)-8) {
		return nil, errCorrupt
	}
	d = &Decoder{IDBase: idBase, FileBase: fileBase, HygBase: hygBase, data: data, p: 8, end: 8 + int(n)}
	// string table
	t := &Decoder{data: data, p: d.end, end: len(data)}
	ns := t.u()
	if ns > uint64(len(data)) {
		return nil, errCorrupt
	}
	lens := make([]int, ns)
	total := 0
	for i := range lens {
		l := t.u()
		if l > uint64(len(data)) {
			return nil, errCorrupt
		}
		lens[i] = int(l)
		total += int(l)
	}
	if t.p+total != len(data) {
		return nil, errCorrupt
	}
	blob := string(data[t.p:])
	d.strs = make([]string, ns)
	off := 0
	for i, l := range lens {
		d.strs[i] = blob[off : off+l]
		off += l
	}
	return d, nil
}

// guard runs f, turning decoding panics (corrupt or truncated data) into
// an error.
func (d *Decoder) guard(f func()) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("%w (%v)", errCorrupt, r)
		}
	}()
	f()
	return nil
}

func (d *Decoder) u() uint64 {
	if b := d.data[d.p]; b < 0x80 {
		d.p++
		return uint64(b)
	}
	return d.uSlow()
}

func (d *Decoder) uSlow() uint64 {
	v, n := binary.Uvarint(d.data[d.p:d.end])
	if n <= 0 {
		panic(errCorrupt)
	}
	d.p += n
	return v
}

func (d *Decoder) i() int64 {
	x := d.u()
	return int64(x>>1) ^ -int64(x&1)
}

func (d *Decoder) b() bool { return d.u() != 0 }

func (d *Decoder) s() string { return d.strs[d.u()] }

// length checks a slice length read from the data.
func (d *Decoder) length(n uint64) int {
	if n > uint64(d.end-d.p) {
		panic(errCorrupt)
	}
	return int(n)
}

func (d *Decoder) id() NodeID {
	x := d.u()
	if x == 0 {
		return 0
	}
	return d.IDBase + NodeID(x)
}

func (d *Decoder) hyg() uint32 {
	x := d.u()
	if x == 0 {
		return 0
	}
	return d.HygBase + uint32(x)
}

func (d *Decoder) pos() lexer.Pos {
	var p lexer.Pos
	switch f := d.u(); f {
	case 0:
		return p
	case 1:
		p.File = d.last.File
		p.Off = d.last.Off + int32(d.i())
		p.Line = d.last.Line + int32(d.i())
		p.Col = int32(d.u())
	default:
		p.File = int32(d.FileBase + int(f) - 2)
		p.Off = int32(d.u())
		p.Line = int32(d.u())
		p.Col = int32(d.u())
	}
	d.last = p
	return p
}

func (d *Decoder) span() Span {
	lo := d.pos()
	switch d.u() {
	case 0:
		return Span{Lo: lo, Hi: lo}
	case 1:
		return Span{Lo: lo, Hi: lexer.Pos{File: lo.File, Off: lo.Off + int32(d.i()), Line: lo.Line + int32(d.i()), Col: int32(d.u())}}
	}
	last := d.last
	hi := d.pos()
	d.last = last
	return Span{Lo: lo, Hi: hi}
}

func (d *Decoder) tok(t *Token) {
	t.Text = d.s()
	t.Suffix = d.s()
	t.Span = d.span()
	t.Hyg = d.hyg()
	t.Kind = lexer.Kind(d.u())
	fl := d.u()
	t.Raw = fl&1 != 0
	t.Inner = fl&2 != 0
	t.JointNext = fl&4 != 0
}

// Crate decodes a crate written by Encoder.Crate.
func (d *Decoder) Crate() (c *Crate, err error) {
	c = &Crate{}
	err = d.guard(func() {
		c.Attrs = d.decSl_Pt_Attr()
		c.Items = d.decSl_Pt_Item()
		c.Span = d.span()
		c.Dir = d.s()
	})
	return c, err
}

// TokenTrees decodes token trees.
func (d *Decoder) TokenTrees() (tts []TokenTree, err error) {
	err = d.guard(func() { tts = d.decSl_TokenTree() })
	return tts, err
}

// String decodes a string.
func (d *Decoder) String() (s string, err error) {
	err = d.guard(func() { s = d.s() })
	return s, err
}

// Uint decodes an unsigned integer.
func (d *Decoder) Uint() (x uint64, err error) {
	err = d.guard(func() { x = d.u() })
	return x, err
}

// Bool decodes a boolean.
func (d *Decoder) Bool() (x bool, err error) {
	err = d.guard(func() { x = d.b() })
	return x, err
}

// Span decodes a span.
func (d *Decoder) Span() (s Span, err error) {
	err = d.guard(func() { s = d.span() })
	return s, err
}

// Done reports whether all values were decoded.
func (d *Decoder) Done() bool { return d.p == d.end }

// NextID returns the last allocated node ID; SetNextID sets it (so that
// NewID continues after a decoded crate's IDs).
func NextID() NodeID      { return nextID }
func SetNextID(id NodeID) { nextID = id }
