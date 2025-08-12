use super::code_indenter::CodeIndenter;
use super::util::{collect_case, print_auto_generated_file_comment, type_ref_name};
use super::Lang;
use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_schema::def::{ModuleDef, ReducerDef, ScopedTypeName, TableDef, TypeDef};
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};
use std::ops::Deref;

/// Code generator for Swift clients.
pub struct Swift;

impl Lang for Swift {
    fn table_filename(&self, _module: &ModuleDef, table: &TableDef) -> String {
        format!("Tables/{}.swift", table.name.deref().to_case(Case::Pascal))
    }

    fn type_filename(&self, type_name: &ScopedTypeName) -> String {
        format!("Types/{}.swift", collect_case(Case::Pascal, type_name.name_segments()))
    }

    fn reducer_filename(&self, reducer_name: &Identifier) -> String {
        format!("Reducers/{}.swift", reducer_name.deref().to_case(Case::Pascal))
    }

    fn generate_table(&self, module: &ModuleDef, tbl: &TableDef) -> String {
        let mut out = CodeIndenter::new(String::new(), INDENT);
        print_auto_generated_file_comment(&mut out);

        let table_name = tbl.name.deref().to_case(Case::Pascal) + "Row";
        writeln!(out, "public struct {table_name} {{");
        out.with_indent(|out| {
            let product = module.typespace_for_generate()[tbl.product_type_ref]
                .as_product()
                .expect("table rows are products");
            for (ident, ty) in &product.elements {
                let field_name = ident.deref().to_case(Case::Camel);
                let ty = swift_type(module, ty);
                writeln!(out, "public var {field_name}: {ty}");
            }
        });
        writeln!(out, "}}");

        out.into_inner()
    }

    fn generate_type(&self, module: &ModuleDef, typ: &TypeDef) -> String {
        let mut out = CodeIndenter::new(String::new(), INDENT);
        print_auto_generated_file_comment(&mut out);

        let type_name = collect_case(Case::Pascal, typ.name.name_segments());
        match &module.typespace_for_generate()[typ.ty] {
            AlgebraicTypeDef::Product(prod) => {
                writeln!(out, "public struct {type_name} {{");
                out.with_indent(|out| {
                    for (ident, ty) in &prod.elements {
                        let field_name = ident.deref().to_case(Case::Camel);
                        let ty = swift_type(module, ty);
                        writeln!(out, "public var {field_name}: {ty}");
                    }
                });
                writeln!(out, "}}");
            }
            AlgebraicTypeDef::Sum(sum) => {
                writeln!(out, "public enum {type_name} {{");
                out.with_indent(|out| {
                    for (ident, ty) in &sum.variants {
                        let case_name = ident.deref().to_case(Case::Camel);
                        match ty {
                            AlgebraicTypeUse::Unit => {
                                writeln!(out, "case {case_name}");
                            }
                            _ => {
                                let ty = swift_type(module, ty);
                                writeln!(out, "case {case_name}({ty})");
                            }
                        }
                    }
                });
                writeln!(out, "}}");
            }
            AlgebraicTypeDef::PlainEnum(e) => {
                writeln!(out, "public enum {type_name}: Int32 {{");
                out.with_indent(|out| {
                    for (idx, ident) in e.variants.iter().enumerate() {
                        let case_name = ident.deref().to_case(Case::Camel);
                        writeln!(out, "case {case_name} = {idx}");
                    }
                });
                writeln!(out, "}}");
            }
        }

        out.into_inner()
    }

    fn generate_reducer(&self, module: &ModuleDef, reducer: &ReducerDef) -> String {
        let mut out = CodeIndenter::new(String::new(), INDENT);
        print_auto_generated_file_comment(&mut out);

        let fn_name = reducer.name.deref().to_case(Case::Camel);
        write!(out, "public func {fn_name}(");
        {
            let mut first = true;
            for (ident, ty) in &reducer.params_for_generate.elements {
                if !first {
                    write!(out, ", ");
                }
                first = false;
                let arg_name = ident.deref().to_case(Case::Camel);
                let ty = swift_type(module, ty);
                write!(out, "{arg_name}: {ty}");
            }
        }
        writeln!(out, ") {{");
        out.with_indent(|out| {
            writeln!(out, "// TODO: call reducer '{fn_name}'");
        });
        writeln!(out, "}}");

        out.into_inner()
    }

    fn generate_globals(&self, _module: &ModuleDef) -> Vec<(String, String)> {
        Vec::new()
    }
}

const INDENT: &str = "    ";

fn swift_type(module: &ModuleDef, ty: &AlgebraicTypeUse) -> String {
    use AlgebraicTypeUse::*;
    match ty {
        Primitive(p) => match p {
            PrimitiveType::Bool => "Bool".into(),
            PrimitiveType::I8 => "Int8".into(),
            PrimitiveType::I16 => "Int16".into(),
            PrimitiveType::I32 => "Int32".into(),
            PrimitiveType::I64 => "Int64".into(),
            PrimitiveType::U8 => "UInt8".into(),
            PrimitiveType::U16 => "UInt16".into(),
            PrimitiveType::U32 => "UInt32".into(),
            PrimitiveType::U64 => "UInt64".into(),
            PrimitiveType::F32 => "Float".into(),
            PrimitiveType::F64 => "Double".into(),
            PrimitiveType::I128 => "Int128".into(),
            PrimitiveType::U128 => "UInt128".into(),
            PrimitiveType::I256 => "Int256".into(),
            PrimitiveType::U256 => "UInt256".into(),
        },
        String => "String".into(),
        Identity => "Identity".into(),
        ConnectionId => "ConnectionId".into(),
        Array(inner) => format!("[{}]", swift_type(module, inner)),
        Option(inner) => format!("{}?", swift_type(module, inner)),
        Ref(r) => type_ref_name(module, *r),
        Unit => "Void".into(),
        _ => "TODO".into(),
    }
}
