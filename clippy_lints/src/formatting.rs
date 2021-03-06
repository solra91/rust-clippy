use rustc::lint::*;
use syntax::codemap::mk_sp;
use syntax::ast;
use utils::{differing_macro_contexts, in_macro, snippet_opt, span_note_and_lint};
use syntax::ptr::P;

/// **What it does:** Checks for use of the non-existent `=*`, `=!` and `=-` operators.
///
/// **Why is this bad?** This is either a typo of `*=`, `!=` or `-=` or confusing.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust,ignore
/// a =- 42; // confusing, should it be `a -= 42` or `a = -42`?
/// ```
declare_lint! {
    pub SUSPICIOUS_ASSIGNMENT_FORMATTING,
    Warn,
    "suspicious formatting of `*=`, `-=` or `!=`"
}

/// **What it does:** Checks for formatting of `else if`. It lints if the `else`
/// and `if` are not on the same line or the `else` seems to be missing.
///
/// **Why is this bad?** This is probably some refactoring remnant, even if the
/// code is correct, it might look confusing.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust,ignore
/// if foo {
/// } if bar { // looks like an `else` is missing here
/// }
///
/// if foo {
/// } else
///
/// if bar { // this is the `else` block of the previous `if`, but should it be?
/// }
/// ```
declare_lint! {
    pub SUSPICIOUS_ELSE_FORMATTING,
    Warn,
    "suspicious formatting of `else if`"
}

#[derive(Copy,Clone)]
pub struct Formatting;

impl LintPass for Formatting {
    fn get_lints(&self) -> LintArray {
        lint_array![SUSPICIOUS_ASSIGNMENT_FORMATTING, SUSPICIOUS_ELSE_FORMATTING]
    }
}

impl EarlyLintPass for Formatting {
    fn check_block(&mut self, cx: &EarlyContext, block: &ast::Block) {
        for w in block.stmts.windows(2) {
            match (&w[0].node, &w[1].node) {
                (&ast::StmtKind::Expr(ref first), &ast::StmtKind::Expr(ref second)) |
                (&ast::StmtKind::Expr(ref first), &ast::StmtKind::Semi(ref second)) => {
                    check_consecutive_ifs(cx, first, second);
                },
                _ => (),
            }
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext, expr: &ast::Expr) {
        check_assign(cx, expr);
        check_else_if(cx, expr);
    }
}

/// Implementation of the `SUSPICIOUS_ASSIGNMENT_FORMATTING` lint.
fn check_assign(cx: &EarlyContext, expr: &ast::Expr) {
    if let ast::ExprKind::Assign(ref lhs, ref rhs) = expr.node {
        if !differing_macro_contexts(lhs.span, rhs.span) && !in_macro(cx, lhs.span) {
            let eq_span = mk_sp(lhs.span.hi, rhs.span.lo);

            if let ast::ExprKind::Unary(op, ref sub_rhs) = rhs.node {
                if let Some(eq_snippet) = snippet_opt(cx, eq_span) {
                    let op = ast::UnOp::to_string(op);
                    let eqop_span = mk_sp(lhs.span.hi, sub_rhs.span.lo);
                    if eq_snippet.ends_with('=') {
                        span_note_and_lint(cx,
                                           SUSPICIOUS_ASSIGNMENT_FORMATTING,
                                           eqop_span,
                                           &format!("this looks like you are trying to use `.. {op}= ..`, but you \
                                                     really are doing `.. = ({op} ..)`",
                                                    op = op),
                                           eqop_span,
                                           &format!("to remove this lint, use either `{op}=` or `= {op}`", op = op));
                    }
                }
            }
        }
    }
}

/// Implementation of the `SUSPICIOUS_ELSE_FORMATTING` lint for weird `else if`.
fn check_else_if(cx: &EarlyContext, expr: &ast::Expr) {
    if let Some((then, &Some(ref else_))) = unsugar_if(expr) {
        if unsugar_if(else_).is_some() && !differing_macro_contexts(then.span, else_.span) && !in_macro(cx, then.span) {
            // this will be a span from the closing ‘}’ of the “then” block (excluding) to the
            // “if” of the “else if” block (excluding)
            let else_span = mk_sp(then.span.hi, else_.span.lo);

            // the snippet should look like " else \n    " with maybe comments anywhere
            // it’s bad when there is a ‘\n’ after the “else”
            if let Some(else_snippet) = snippet_opt(cx, else_span) {
                let else_pos = else_snippet.find("else").expect("there must be a `else` here");

                if else_snippet[else_pos..].contains('\n') {
                    span_note_and_lint(cx,
                                       SUSPICIOUS_ELSE_FORMATTING,
                                       else_span,
                                       "this is an `else if` but the formatting might hide it",
                                       else_span,
                                       "to remove this lint, remove the `else` or remove the new line between `else` \
                                        and `if`");
                }
            }
        }
    }
}

/// Implementation of the `SUSPICIOUS_ELSE_FORMATTING` lint for consecutive ifs.
fn check_consecutive_ifs(cx: &EarlyContext, first: &ast::Expr, second: &ast::Expr) {
    if !differing_macro_contexts(first.span, second.span) && !in_macro(cx, first.span) &&
       unsugar_if(first).is_some() && unsugar_if(second).is_some() {
        // where the else would be
        let else_span = mk_sp(first.span.hi, second.span.lo);

        if let Some(else_snippet) = snippet_opt(cx, else_span) {
            if !else_snippet.contains('\n') {
                span_note_and_lint(cx,
                                   SUSPICIOUS_ELSE_FORMATTING,
                                   else_span,
                                   "this looks like an `else if` but the `else` is missing",
                                   else_span,
                                   "to remove this lint, add the missing `else` or add a new line before the second \
                                    `if`");
            }
        }
    }
}

/// Match `if` or `if let` expressions and return the `then` and `else` block.
fn unsugar_if(expr: &ast::Expr) -> Option<(&P<ast::Block>, &Option<P<ast::Expr>>)> {
    match expr.node {
        ast::ExprKind::If(_, ref then, ref else_) |
        ast::ExprKind::IfLet(_, _, ref then, ref else_) => Some((then, else_)),
        _ => None,
    }
}
