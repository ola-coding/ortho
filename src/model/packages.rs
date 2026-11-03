use crate::parser::ast::{Ast, NodeId, Prop, Type};

/// The given packages and every package nested in them.
pub fn collect_packages(ast: &Ast, roots: &[NodeId]) -> Vec<NodeId> {
    fn visit(ast: &Ast, package: NodeId, packages: &mut Vec<NodeId>) {
        packages.push(package);
        for member in ast.children(package, Prop::Members) {
            if ast[member].ty == Type::PackageDecl {
                visit(ast, member, packages);
            }
        }
    }
    let mut packages = Vec::new();
    for root in roots {
        visit(ast, *root, &mut packages);
    }
    packages
}

/// What a view presents of the elements it exposes: an exposed package stands for
/// its members, nested packages' included, and anything else for itself.
pub fn exposed_members(ast: &Ast, exposed: &[NodeId]) -> Vec<NodeId> {
    let (packages, others): (Vec<NodeId>, Vec<NodeId>) =
        exposed.iter().partition(|element| ast[**element].ty == Type::PackageDecl);
    let members = collect_packages(ast, &packages).into_iter().flat_map(|package| ast.children(package, Prop::Members));
    others.into_iter().chain(members).collect()
}
