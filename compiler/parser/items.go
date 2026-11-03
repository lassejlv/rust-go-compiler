package parser

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

func (p *Parser) parseItem() *ast.Item {
	attrs := p.parseOuterAttrs()
	lo := p.lo()
	if len(attrs) > 0 {
		lo = attrs[0].Span.Lo
	}
	return p.parseItemAfterAttrs(lo, attrs)
}

type fnQuals struct {
	Const, Async, Unsafe bool
	Abi                  string
	hasAbi               bool
}

// parseFnQuals parses `const async unsafe extern "abi"` before `fn`.
// It returns ok == false (without consuming anything) if no `fn` follows.
func (p *Parser) parseFnQuals() (q fnQuals, ok bool) {
	save := p.Pos()
	saveLast := p.lastHi
	for {
		switch {
		case p.is("const"):
			q.Const = true
		case p.is("async"):
			q.Async = true
		case p.is("unsafe"):
			q.Unsafe = true
		case p.is("safe") && !p.cur().Raw:
			// `safe fn` in extern blocks
		case p.is("default") && (p.isN(1, "fn") || p.isN(1, "unsafe") || p.isN(1, "const") || p.isN(1, "async")):
		case p.is("extern"):
			p.next()
			q.hasAbi = true
			q.Abi = "C"
			if p.cur().Kind == lexer.Str {
				q.Abi = p.cur().Text
			} else {
				continue
			}
		case p.is("fn"):
			return q, true
		default:
			p.Reset(save)
			p.lastHi = saveLast
			return fnQuals{}, false
		}
		p.next()
	}
}

func (p *Parser) parseItemAfterAttrs(lo lexer.Pos, attrs []*ast.Attr) *ast.Item {
	it := &ast.Item{Attrs: attrs}
	start := p.idx
	it.Vis = p.parseVis()
	t := p.cur()
	finish := func(kind ast.ItemKind) *ast.Item {
		it.Kind = kind
		it.Span = p.span(lo)
		it.ID = ast.NewID()
		if p.idx > start && p.idx <= len(p.toks) {
			it.Tokens = p.toks[start:p.idx:p.idx]
		}
		return it
	}
	if q, ok := p.parseFnQuals(); ok {
		return finish(p.parseFn(it, q))
	}
	if t.Kind == lexer.Ident && !t.Raw {
		switch t.Text {
		case "use":
			p.next()
			tree := p.parseUseTree()
			p.expect(";")
			return finish(&ast.UseItem{Tree: tree})
		case "struct":
			p.next()
			return finish(p.parseStruct(it))
		case "union":
			if isPlainIdent(p.peekN(1)) {
				p.next()
				return finish(p.parseUnion(it))
			}
		case "enum":
			p.next()
			return finish(p.parseEnum(it))
		case "trait":
			return finish(p.parseTrait(it, false, false))
		case "auto":
			if p.isN(1, "trait") {
				p.next()
				return finish(p.parseTrait(it, false, true))
			}
		case "impl":
			return finish(p.parseImpl(false))
		case "unsafe":
			switch {
			case p.isN(1, "impl"):
				p.next()
				return finish(p.parseImpl(true))
			case p.isN(1, "trait"):
				p.next()
				return finish(p.parseTrait(it, true, false))
			case p.isN(1, "auto") && p.isN(2, "trait"):
				p.next()
				p.next()
				return finish(p.parseTrait(it, true, true))
			case p.isN(1, "extern"):
				p.next()
				return finish(p.parseExternBlock())
			case p.isN(1, "mod"):
				p.next()
				return finish(p.parseMod(it))
			}
		case "type":
			p.next()
			return finish(p.parseTypeAlias(it))
		case "const":
			p.next()
			return finish(p.parseConst(it))
		case "static":
			p.next()
			return finish(p.parseStatic(it))
		case "mod":
			return finish(p.parseMod(it))
		case "extern":
			if p.isN(1, "crate") {
				p.next()
				p.next()
				if p.is("self") {
					t := p.next()
					it.Name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
				} else {
					it.Name = p.parseIdentTok()
				}
				ec := &ast.ExternCrateItem{}
				if p.eat("as") {
					var id ast.Ident
					if p.is("_") {
						t := p.next()
						id = ast.Ident{Name: "_", Span: t.Span}
					} else {
						id = p.parseIdentTok()
					}
					ec.Rename = &id
				}
				p.expect(";")
				return finish(ec)
			}
			return finish(p.parseExternBlock())
		case "macro_rules":
			if p.isN(1, "!") {
				p.next()
				p.next()
				it.Name = p.parseAnyIdent()
				g := p.parseDelimGroup()
				if g.Delim != '{' {
					p.expect(";")
				} else {
					p.eat(";")
				}
				return finish(&ast.MacroRulesItem{Body: g.Inner})
			}
		}
	}
	if p.isMacroCallStart() {
		mac := p.parseMacCall()
		if mac.Delim != '{' {
			p.expect(";")
		} else {
			p.eat(";")
		}
		return finish(&ast.MacCallItem{Mac: mac})
	}
	p.errExpected("item")
	return nil
}

// parseAnyIdent accepts any identifier, including keywords.
func (p *Parser) parseAnyIdent() ast.Ident {
	t := p.cur()
	if t.Kind != lexer.Ident {
		p.errExpected("identifier")
	}
	p.next()
	return ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
}

func (p *Parser) parseFn(it *ast.Item, q fnQuals) *ast.FnItem {
	p.expect("fn")
	it.Name = p.parseIdentTok()
	fn := &ast.FnItem{Const: q.Const, Async: q.Async, Unsafe: q.Unsafe}
	if q.hasAbi {
		fn.Abi = q.Abi
	}
	fn.Generics = p.parseGenericParams()
	fn.Decl = p.parseFnDecl()
	p.parseWhere(fn.Generics)
	if !p.eat(";") {
		fn.Body = p.parseBlock()
	}
	return fn
}

func (p *Parser) isSelfParam() bool {
	isSelf := func(n int) bool { return p.isN(n, "self") && !p.isN(n+1, "::") }
	switch {
	case isSelf(0):
		return true
	case p.is("mut"):
		return isSelf(1)
	case p.is("&"):
		n := 1
		if p.peekN(n).Kind == lexer.Lifetime {
			n++
		}
		if p.isN(n, "mut") {
			n++
		}
		return isSelf(n)
	}
	return false
}

func (p *Parser) parseSelfParam() *ast.SelfParam {
	lo := p.lo()
	sp := &ast.SelfParam{}
	if p.eat("&") {
		sp.Kind = ast.SelfRef
		if p.cur().Kind == lexer.Lifetime {
			sp.Lifetime = p.next().Text
		}
		if p.eat("mut") {
			sp.Kind = ast.SelfRefMut
		}
		p.expect("self")
	} else {
		sp.Mut = p.eat("mut")
		p.expect("self")
		sp.Kind = ast.SelfValue
		if p.eat(":") {
			sp.Kind = ast.SelfTyped
			sp.Ty = p.parseType()
		}
	}
	sp.Span = p.span(lo)
	sp.ID = ast.NewID()
	return sp
}

func (p *Parser) parseFnDecl() *ast.FnDecl {
	p.expect("(")
	decl := &ast.FnDecl{}
	first := true
	for !p.is(")") {
		attrs := p.parseOuterAttrs()
		lo := p.lo()
		if len(attrs) > 0 {
			lo = attrs[0].Span.Lo
		}
		switch {
		case first && p.isSelfParam():
			decl.SelfParam = p.parseSelfParam()
		case p.is("..."):
			p.next()
			decl.Variadic = true
		default:
			pat := p.parsePatNoTopAlt()
			p.expect(":")
			if p.eat("...") {
				decl.Variadic = true
			} else {
				ty := p.parseType()
				decl.Params = append(decl.Params, &ast.Param{Attrs: attrs, Pat: pat, Ty: ty, Span: p.span(lo)})
			}
		}
		first = false
		if !p.eat(",") {
			break
		}
	}
	p.expect(")")
	if p.eat("->") {
		decl.Output = p.parseType()
	}
	return decl
}

func (p *Parser) parseNamedFields() []*ast.FieldDef {
	p.expect("{")
	var fields []*ast.FieldDef
	for !p.is("}") {
		attrs := p.parseOuterAttrs()
		lo := p.lo()
		if len(attrs) > 0 {
			lo = attrs[0].Span.Lo
		}
		fd := &ast.FieldDef{Attrs: attrs}
		fd.Vis = p.parseVis()
		fd.Name = p.parseIdentTok()
		p.expect(":")
		fd.Ty = p.parseType()
		fd.Span = p.span(lo)
		fd.ID = ast.NewID()
		fields = append(fields, fd)
		if !p.eat(",") {
			break
		}
	}
	p.expect("}")
	return fields
}

func (p *Parser) parseTupleFields() []*ast.FieldDef {
	p.expect("(")
	var fields []*ast.FieldDef
	for !p.is(")") {
		attrs := p.parseOuterAttrs()
		lo := p.lo()
		if len(attrs) > 0 {
			lo = attrs[0].Span.Lo
		}
		fd := &ast.FieldDef{Attrs: attrs}
		fd.Vis = p.parseVis()
		fd.Ty = p.parseType()
		fd.Span = p.span(lo)
		fd.ID = ast.NewID()
		fields = append(fields, fd)
		if !p.eat(",") {
			break
		}
	}
	p.expect(")")
	return fields
}

func (p *Parser) parseStruct(it *ast.Item) *ast.StructItem {
	it.Name = p.parseIdentTok()
	s := &ast.StructItem{Generics: p.parseGenericParams()}
	p.parseWhere(s.Generics)
	switch {
	case p.is("{"):
		s.Shape = ast.ShapeNamed
		s.Fields = p.parseNamedFields()
	case p.is("("):
		s.Shape = ast.ShapeTuple
		s.Fields = p.parseTupleFields()
		p.parseWhere(s.Generics)
		p.expect(";")
	default:
		s.Shape = ast.ShapeUnit
		p.expect(";")
	}
	return s
}

func (p *Parser) parseUnion(it *ast.Item) *ast.UnionItem {
	it.Name = p.parseIdentTok()
	u := &ast.UnionItem{Generics: p.parseGenericParams()}
	p.parseWhere(u.Generics)
	u.Fields = p.parseNamedFields()
	return u
}

func (p *Parser) parseEnum(it *ast.Item) *ast.EnumItem {
	it.Name = p.parseIdentTok()
	e := &ast.EnumItem{Generics: p.parseGenericParams()}
	p.parseWhere(e.Generics)
	p.expect("{")
	for !p.is("}") {
		attrs := p.parseOuterAttrs()
		lo := p.lo()
		if len(attrs) > 0 {
			lo = attrs[0].Span.Lo
		}
		v := &ast.Variant{Attrs: attrs}
		p.parseVis()
		v.Name = p.parseIdentTok()
		switch {
		case p.is("{"):
			v.Shape = ast.ShapeNamed
			v.Fields = p.parseNamedFields()
		case p.is("("):
			v.Shape = ast.ShapeTuple
			v.Fields = p.parseTupleFields()
		default:
			v.Shape = ast.ShapeUnit
		}
		if p.eat("=") {
			v.Disc = p.parseExpr()
		}
		v.Span = p.span(lo)
		v.ID = ast.NewID()
		e.Variants = append(e.Variants, v)
		if !p.eat(",") {
			break
		}
	}
	p.expect("}")
	return e
}

func (p *Parser) parseTrait(it *ast.Item, unsafe, auto bool) *ast.TraitItem {
	p.expect("trait")
	it.Name = p.parseIdentTok()
	tr := &ast.TraitItem{Unsafe: unsafe, Auto: auto}
	tr.Generics = p.parseGenericParams()
	if p.eat(":") {
		tr.Supertraits = p.parseBounds(true)
	}
	p.parseWhere(tr.Generics)
	if p.eat("=") {
		// trait alias: trait A = B + C;
		tr.Supertraits = p.parseBounds(true)
		p.parseWhere(tr.Generics)
		p.expect(";")
		return tr
	}
	p.expect("{")
	it.Attrs = append(it.Attrs, p.parseInnerAttrs()...)
	for !p.is("}") {
		if p.eat(";") {
			continue
		}
		tr.Items = append(tr.Items, p.parseAssocItem())
	}
	p.expect("}")
	return tr
}

// looksLikeImplGenerics distinguishes `impl<T> ...` from `impl <T as X>::Y`.
func (p *Parser) looksLikeImplGenerics() bool {
	if !p.is("<") {
		return false
	}
	t1 := p.peekN(1)
	if t1.Is(">") || t1.Kind == lexer.Lifetime || t1.Is("const") || t1.Is("#") {
		return true
	}
	if isPlainIdent(t1) {
		t2 := p.peekN(2)
		return t2.Is(">") || t2.Is(",") || t2.Is(":") || t2.Is("=")
	}
	return false
}

func (p *Parser) parseImpl(unsafe bool) *ast.ImplItem {
	p.expect("impl")
	im := &ast.ImplItem{Unsafe: unsafe}
	if p.looksLikeImplGenerics() {
		im.Generics = p.parseGenericParams()
	} else {
		im.Generics = &ast.Generics{Span: lexer.Span{Lo: p.lo(), Hi: p.lo()}}
	}
	p.eat("const")
	if p.is("!") {
		p.next()
		im.Negative = true
	}
	ty := p.parseTypeNoPlus()
	if p.eat("for") {
		switch tt := ty.(type) {
		case *ast.PathType:
			if tt.QSelf != nil {
				p.errorf("expected a trait path in impl")
			}
			im.Trait = tt.Path
		case *ast.DynTraitType:
			if len(tt.Bounds) == 1 && tt.Bounds[0].Path != nil {
				im.Trait = tt.Bounds[0].Path
			} else {
				p.errorf("expected a trait path in impl")
			}
		default:
			p.errorf("expected a trait path in impl")
		}
		im.SelfTy = p.parseType()
	} else {
		// `impl dyn Trait + Send { .. }`
		if dt, ok := ty.(*ast.DynTraitType); ok {
			for p.eat("+") {
				dt.Bounds = append(dt.Bounds, p.parseBound())
			}
		}
		im.SelfTy = ty
	}
	p.parseWhere(im.Generics)
	p.expect("{")
	p.parseInnerAttrs()
	for !p.is("}") {
		if p.eat(";") {
			continue
		}
		im.Items = append(im.Items, p.parseAssocItem())
	}
	p.expect("}")
	return im
}

// parseAssocItem parses an item inside a trait or impl body.
func (p *Parser) parseAssocItem() *ast.Item {
	attrs := p.parseOuterAttrs()
	lo := p.lo()
	if len(attrs) > 0 {
		lo = attrs[0].Span.Lo
	}
	it := &ast.Item{Attrs: attrs}
	it.Vis = p.parseVis()
	if p.is("default") && !p.cur().Raw && (p.isN(1, "type") || p.isN(1, "const") || p.isN(1, "fn") || p.isN(1, "unsafe") || p.isN(1, "async") || p.isN(1, "extern")) {
		p.next()
	}
	finish := func(kind ast.ItemKind) *ast.Item {
		it.Kind = kind
		it.Span = p.span(lo)
		it.ID = ast.NewID()
		return it
	}
	if q, ok := p.parseFnQuals(); ok {
		return finish(p.parseFn(it, q))
	}
	switch {
	case p.is("type"):
		p.next()
		return finish(p.parseTypeAlias(it))
	case p.is("const"):
		p.next()
		return finish(p.parseConst(it))
	case p.isMacroCallStart():
		mac := p.parseMacCall()
		if mac.Delim != '{' {
			p.expect(";")
		} else {
			p.eat(";")
		}
		return finish(&ast.MacCallItem{Mac: mac})
	}
	p.errExpected("associated item (`fn`, `type` or `const`)")
	return nil
}

func (p *Parser) parseTypeAlias(it *ast.Item) *ast.TypeAliasItem {
	it.Name = p.parseIdentTok()
	ta := &ast.TypeAliasItem{Generics: p.parseGenericParams()}
	if p.eat(":") {
		ta.Bounds = p.parseBounds(true)
	}
	p.parseWhere(ta.Generics)
	if p.eat("=") {
		ta.Ty = p.parseType()
	}
	p.parseWhere(ta.Generics)
	p.expect(";")
	return ta
}

func (p *Parser) parseConst(it *ast.Item) *ast.ConstItem {
	if p.is("_") {
		t := p.next()
		it.Name = ast.Ident{Name: "_", Span: t.Span}
	} else {
		it.Name = p.parseIdentTok()
	}
	c := &ast.ConstItem{}
	if p.eat(":") {
		c.Ty = p.parseType()
	}
	if p.eat("=") {
		c.Expr = p.parseExpr()
	}
	p.expect(";")
	return c
}

func (p *Parser) parseStatic(it *ast.Item) *ast.StaticItem {
	s := &ast.StaticItem{}
	s.Mut = p.eat("mut")
	it.Name = p.parseIdentTok()
	p.expect(":")
	s.Ty = p.parseType()
	if p.eat("=") {
		s.Expr = p.parseExpr()
	}
	p.expect(";")
	return s
}

func (p *Parser) parseMod(it *ast.Item) *ast.ModItem {
	p.expect("mod")
	it.Name = p.parseIdentTok()
	m := &ast.ModItem{}
	if p.eat(";") {
		return m
	}
	m.Inline = true
	p.expect("{")
	it.Attrs = append(it.Attrs, p.parseInnerAttrs()...)
	for !p.is("}") {
		if p.atEOF() {
			p.errExpected("`}`")
		}
		if p.eat(";") {
			continue
		}
		m.Items = append(m.Items, p.parseItem())
	}
	p.expect("}")
	return m
}

func (p *Parser) parseExternBlock() *ast.ExternBlockItem {
	p.expect("extern")
	eb := &ast.ExternBlockItem{Abi: "C"}
	if p.cur().Kind == lexer.Str {
		eb.Abi = p.next().Text
	}
	p.expect("{")
	p.parseInnerAttrs()
	for !p.is("}") {
		if p.atEOF() {
			p.errExpected("`}`")
		}
		if p.eat(";") {
			continue
		}
		attrs := p.parseOuterAttrs()
		lo := p.lo()
		if len(attrs) > 0 {
			lo = attrs[0].Span.Lo
		}
		// `safe`/`unsafe` qualifiers on statics
		if (p.is("safe") || p.is("unsafe")) && p.isN(1, "static") {
			p.next()
		}
		eb.Items = append(eb.Items, p.parseItemAfterAttrs(lo, attrs))
	}
	p.expect("}")
	return eb
}

// ---------------------------------------------------------------------------
// Use trees

func (p *Parser) parseUseTree() *ast.UseTree {
	lo := p.lo()
	ut := &ast.UseTree{}
	prefix := &ast.Path{}
	if p.is("::") {
		p.next()
		prefix.Global = true
	}
	for {
		if p.is("*") {
			p.next()
			ut.Kind = ast.UseGlob
			break
		}
		if p.is("{") {
			p.next()
			ut.Kind = ast.UseNested
			for !p.is("}") {
				ut.Nested = append(ut.Nested, p.parseUseTree())
				if !p.eat(",") {
					break
				}
			}
			p.expect("}")
			break
		}
		seg := p.parsePathSegIdent()
		prefix.Segments = append(prefix.Segments, &ast.PathSegment{Ident: seg})
		if p.eat("::") {
			continue
		}
		ut.Kind = ast.UseSimple
		if p.eat("as") {
			var id ast.Ident
			if p.is("_") {
				t := p.next()
				id = ast.Ident{Name: "_", Span: t.Span}
			} else {
				id = p.parseIdentTok()
			}
			ut.Rename = &id
		}
		break
	}
	prefix.Span = p.span(lo)
	prefix.ID = ast.NewID()
	ut.Prefix = prefix
	ut.Span = p.span(lo)
	ut.ID = ast.NewID()
	return ut
}
