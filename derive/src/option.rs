/// ref: <https://stackoverflow.com/a/56264023/9920172>
pub fn extract_type_from_option(ty: &syn::Type) -> Option<&syn::Type> {
    use syn::{GenericArgument, Path, PathArguments, PathSegment};

    fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
        match ty {
            syn::Type::Path(type_path) if type_path.qself.is_none() => Some(&type_path.path),
            _ => None,
        }
    }

    fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
        let idents = path
            .segments
            .iter()
            .map(|x| x.ident.to_string())
            .collect::<Vec<String>>();
        let idents = idents.iter().map(|x| x.as_str()).collect::<Vec<&str>>();
        if matches!(
            idents.as_slice(),
            ["Option"] | ["std", "option", "Option"] | ["core", "option", "Option"]
        ) {
            return path.segments.last();
        }
        None
    }

    extract_type_path(ty)
        .and_then(extract_option_segment)
        .and_then(|path_seg| {
            // It should have only on angle-bracketed param ("<String>"):
            match &path_seg.arguments {
                PathArguments::AngleBracketed(params) => params.args.first(),
                _ => None,
            }
        })
        .and_then(|generic_arg| match generic_arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
}
