pub fn expand_shader_data_derive(input: syn::DeriveInput) -> proc_macro::TokenStream {
    let ty = &input.ident;

    let data = match input.data {
        syn::Data::Struct(s) => s,
        _ => panic!()
    };

    let fn_size = Vec::new();

    for field in &data.fields {
        
    }

    quote::quote! {
        impl ShaderData for #ty {
            fn size() -> usize {
                std::mem::size_of::<Mat4>() * 2
            }
        
            fn as_raw(&self) -> Vec<u8> {
                let mut buf = Vec::with_capacity(std::mem::size_of::<Self>());
                buf.extend_from_slice(bytemuck::cast_slice(self.view.as_ref()));
                buf.extend_from_slice(bytemuck::cast_slice(self.proj.as_ref()));
                buf
            }
        }
    }
    .into()
}
