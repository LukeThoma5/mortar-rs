use swc_atoms::js_word;
use swc_common::Span;
use swc_ecma_ast::*;

pub fn make_module() -> Module {
    Module {
        shebang: None,
        span: Default::default(),
        body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: Default::default(),
            expr: Box::new(Expr::This(ThisExpr {
                span: Default::default(),
            })),
        }))],
    }
}

pub fn make_example() -> Module {
    Module {
        span: Default::default(),
        body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span: span(),
            decl: Decl::Var(VarDecl {
                span: span(),
                declare: false,
                kind: VarDeclKind::Const,
                decls: vec![make_fn()],
            }),
        }))],
        shebang: None,
    }
}

fn make_fn() -> VarDeclarator {
    // note to use a keyword do
    //let sym = js_word!("enum");
    VarDeclarator {
        span: span(),
        name: Pat::Ident(Ident {
            span: span(),
            optional: false,
            sym: "someActionName".into(),
            type_ann: None,
        }),
        definite: false,
        init: Some(Box::new(Expr::Arrow(ArrowExpr {
            is_async: false,
            is_generator: false,
            type_params: None,
            span: span(),
            return_type: None,
            params: vec![], // TODO
            body: BlockStmtOrExpr::Expr(Box::new(Expr::Call(CallExpr {
                span: span(),
                args: vec![], // TODO
                type_args: Some(TsTypeParamInstantiation {
                    span: span(),
                    params: vec![Box::new(TsType::TsKeywordType(TsKeywordType {
                        span: span(),
                        kind: TsKeywordTypeKind::TsStringKeyword,
                    }))],
                }),
                callee: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                    span: span(),
                    optional: false,
                    sym: "apiGet".into(),
                    type_ann: None,
                }))),
            }))),
        }))),
    }
}

pub fn span() -> Span {
    Default::default()
}
