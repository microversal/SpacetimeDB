use super::code_indenter::CodeIndenter;
use super::util::{collect_case, print_auto_generated_file_comment, type_ref_name};
use super::Lang;
use convert_case::{Case, Casing};
use itertools::Itertools;
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

        let table_name_pascal = tbl.name.deref().to_case(Case::Pascal);
        let row_struct_name = format!("{table_name_pascal}Row");
        writeln!(out, "public struct {row_struct_name}: Codable {{");
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

        writeln!(out);

        let handle_name = format!("{table_name_pascal}TableHandle");
        writeln!(out, "public final class {handle_name} {{");
        out.with_indent(|out| {
            writeln!(out, "private let tableCache: TableCache<{row_struct_name}>");
            writeln!(out, "public init(tableCache: TableCache<{row_struct_name}>) {{");
            out.with_indent(|out| writeln!(out, "self.tableCache = tableCache"));
            writeln!(out, "}}");
            writeln!(out);
            writeln!(out, "public func count() -> Int {{");
            out.with_indent(|out| writeln!(out, "return tableCache.count()"));
            writeln!(out, "}}");
            writeln!(out);
            writeln!(out, "public func rows() -> [{row_struct_name}] {{");
            out.with_indent(|out| writeln!(out, "return tableCache.rows()"));
            writeln!(out, "}}");
            writeln!(out);
            writeln!(
                out,
                "public func onInsert(_ callback: @escaping ({row_struct_name}) -> Void) {{"
            );
            out.with_indent(|out| writeln!(out, "tableCache.onInsert(callback)"));
            writeln!(out, "}}");
            writeln!(out);
            writeln!(
                out,
                "public func onDelete(_ callback: @escaping ({row_struct_name}) -> Void) {{"
            );
            out.with_indent(|out| writeln!(out, "tableCache.onDelete(callback)"));
            writeln!(out, "}}");
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
                writeln!(out, "public struct {type_name}: Codable {{");
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
                writeln!(out, "public enum {type_name}: Codable {{");
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
                writeln!(out, "public enum {type_name}: Int32, Codable {{");
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
        writeln!(out, "import Foundation");
        writeln!(out);

        let fn_name = reducer.name.deref().to_case(Case::Camel);
        let on_fn_name = format!("on{}", reducer.name.deref().to_case(Case::Pascal));
        let reducer_name = reducer.name.deref();

        writeln!(out, "public extension RemoteReducers {{");
        out.with_indent(|out| {
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
                if !reducer.params_for_generate.elements.is_empty() {
                    write!(out, ", ");
                }
                write!(out, "flags: ReducerCallFlags = []");
            }
            writeln!(out, ") {{");
            out.with_indent(|out| {
                if reducer.params_for_generate.elements.is_empty() {
                    writeln!(
                        out,
                        "connection.callReducer(reducer: \"{reducer_name}\", args: Data(), flags: flags)"
                    );
                } else {
                    writeln!(out, "struct Args: Codable {{");
                    out.with_indent(|out| {
                        for (ident, ty) in &reducer.params_for_generate.elements {
                            let arg_name = ident.deref().to_case(Case::Camel);
                            let ty = swift_type(module, ty);
                            writeln!(out, "let {arg_name}: {ty}");
                        }
                    });
                    writeln!(out, "}}");
                    let mut arg_inits = String::new();
                    for (idx, (ident, _)) in reducer.params_for_generate.elements.iter().enumerate() {
                        if idx != 0 {
                            arg_inits.push_str(", ");
                        }
                        let arg_name = ident.deref().to_case(Case::Camel);
                        arg_inits.push_str(&format!("{arg_name}: {arg_name}"));
                    }
                    writeln!(out, "let args = Args({arg_inits})");
                    writeln!(out, "let data = try! JSONEncoder().encode(args)");
                    writeln!(
                        out,
                        "connection.callReducer(reducer: \"{reducer_name}\", args: data, flags: flags)"
                    );
                }
            });
            writeln!(out, "}}");
            writeln!(out);
            writeln!(out, "public func {on_fn_name}(callback: @escaping () -> Void) {{");
            out.with_indent(|out| {
                writeln!(
                    out,
                    "connection.onReducer(reducer: \"{reducer_name}\", callback: callback)"
                );
            });
            writeln!(out, "}}");
        });
        writeln!(out, "}}");

        out.into_inner()
    }

    fn generate_globals(&self, module: &ModuleDef) -> Vec<(String, String)> {
        let mut out = CodeIndenter::new(String::new(), INDENT);
        print_auto_generated_file_comment(&mut out);

        writeln!(out, "public struct ReducerCallFlags: OptionSet {{");
        out.with_indent(|out| {
            writeln!(out, "public let rawValue: Int");
            writeln!(out, "public init(rawValue: Int) {{ self.rawValue = rawValue }}");
        });
        writeln!(out, "}}");
        writeln!(out);

        writeln!(out, "public class RemoteReducers {{");
        out.with_indent(|out| {
            writeln!(out, "let connection: DbConnectionImpl");
            writeln!(out, "public init(connection: DbConnectionImpl) {{");
            out.with_indent(|out| writeln!(out, "self.connection = connection"));
            writeln!(out, "}}");
        });
        writeln!(out, "}}");
        writeln!(out);

        writeln!(out, "public class RemoteTables {{");
        out.with_indent(|out| {
            writeln!(out, "let connection: DbConnectionImpl");
            writeln!(out, "public init(connection: DbConnectionImpl) {{");
            out.with_indent(|out| writeln!(out, "self.connection = connection"));
            writeln!(out, "}}");
            writeln!(out);
            for tbl in module.tables().sorted_by_key(|tbl| &tbl.name) {
                let property_name = tbl.name.deref().to_case(Case::Camel);
                let table_name_pascal = tbl.name.deref().to_case(Case::Pascal);
                let handle_name = format!("{table_name_pascal}TableHandle");
                let row_struct_name = format!("{table_name_pascal}Row");
                writeln!(out, "public lazy var {property_name}: {handle_name} = {handle_name}(tableCache: TableCache<{row_struct_name}>())");
            }
        });
        writeln!(out, "}}");
        writeln!(out);

        writeln!(out, "public final class ModuleDb {{");
        out.with_indent(|out| {
            writeln!(out, "public let connection: DbConnectionImpl");
            writeln!(out, "public let db: RemoteTables");
            writeln!(out, "public let reducers: RemoteReducers");
            writeln!(out, "public init(connection: DbConnectionImpl) {{");
            out.with_indent(|out| {
                writeln!(out, "self.connection = connection");
                writeln!(out, "self.db = RemoteTables(connection: connection)");
                writeln!(out, "self.reducers = RemoteReducers(connection: connection)");
            });
            writeln!(out, "}}");
        });
        writeln!(out, "}}");

        vec![("index.swift".to_string(), out.into_inner())]
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
