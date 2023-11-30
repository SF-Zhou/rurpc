use itertools::Itertools;
use pilota_build::db::RirDatabase;

pub struct MakeBackend;

impl pilota_build::MakeBackend for MakeBackend {
    type Target = ProtoBackend;

    fn make_backend(self, context: pilota_build::Context) -> Self::Target {
        ProtoBackend {
            inner: pilota_build::ProtobufBackend::new(context),
        }
    }
}

#[derive(Clone)]
pub struct ProtoBackend {
    inner: pilota_build::ProtobufBackend,
}

impl pilota_build::CodegenBackend for ProtoBackend {
    fn codegen_service_method(
        &self,
        _service_def_id: pilota_build::DefId,
        method: &pilota_build::rir::Method,
    ) -> String {
        format!(
            r#"
            fn {}(
                &self,
                {}: &{}
            ) -> impl ::std::future::Future<
                Output = ::std::result::Result<
                    {},
                    ::std::boxed::Box<dyn ::std::error::Error>,
                >,
            > + Send;
            "#,
            self.cx().rust_name(method.def_id),
            method.args[0].name,
            self.cx().codegen_item_ty(method.args[0].ty.kind.clone()),
            self.cx().codegen_item_ty(method.ret.kind.clone())
        )
    }

    fn codegen_service_impl(
        &self,
        def_id: pilota_build::DefId,
        stream: &mut String,
        service: &pilota_build::rir::Service,
    ) {
        let service_name = self.cx().rust_name(def_id);
        let server_name = format!("{}Server", service_name);
        let file_id = self.cx().node(def_id).unwrap().file_id;
        let file = self.cx().file(file_id).unwrap();
        let package = file.package.iter().join(".");
        let name = format!("{package}.{}", service.name);
        let req_matches = service
            .methods
            .iter()
            .map(|method| {
                let path = format!("/{package}.{}/{}", service.name, method.name);
                let input_type = self
                    .cx()
                    .codegen_item_ty(method.args[0].ty.kind.clone())
                    .to_string();
                let method_name = self.cx().rust_name(method.def_id);
                format!(
                    r#""{path}" => {{
                        let req = {input_type}::decode(req)?;
                        let rsp = inner.{method_name}(&req).await?;
                        let mut buf = bytes::BytesMut::with_capacity(rsp.encoded_len());
                        let encode_result = rsp.encode(&mut buf)?;
                        Ok(buf.into())
                    }},
                "#
                )
            })
            .join("");

        stream.push_str(&format!(r#"
            pub struct {server_name}<S> {{
                inner: ::std::sync::Arc<S>,
            }}

            impl<S> Clone for {server_name}<S> {{
                fn clone(&self) -> Self {{
                    {server_name} {{
                        inner: self.inner.clone(),
                    }}
                }}
            }}

            impl<S> {server_name}<S> {{
                const NAME: &'static str = "{name}";

                pub fn new(inner: S) -> Self {{
                    Self {{
                        inner: ::std::sync::Arc::new(inner),
                    }}
                }}

                pub async fn call(&self, method: &str, req: impl ::bytes::Buf) -> ::std::result::Result<::bytes::Bytes, ::std::boxed::Box::<dyn ::std::error::Error>>
                where
                    S: {service_name} + ::core::marker::Send + ::core::marker::Sync + 'static,
                {{
                    use pilota::prost::Message;
                    let inner = self.inner.clone();
                    match method {{
                        {req_matches}
                        path => Err(::std::format!("Unimplemented path: {{}}", path).into()),
                    }}
                }}
            }}
        "#));
    }

    fn codegen_enum_impl(
        &self,
        def_id: pilota_build::DefId,
        stream: &mut String,
        e: &pilota_build::rir::Enum,
    ) {
        self.inner.codegen_enum_impl(def_id, stream, e)
    }

    fn codegen_newtype_impl(
        &self,
        def_id: pilota_build::DefId,
        stream: &mut String,
        t: &pilota_build::rir::NewType,
    ) {
        self.inner.codegen_newtype_impl(def_id, stream, t)
    }

    fn codegen_struct_impl(
        &self,
        def_id: pilota_build::DefId,
        stream: &mut String,
        service: &pilota_build::rir::Message,
    ) {
        self.inner.codegen_struct_impl(def_id, stream, service)
    }

    fn cx(&self) -> &pilota_build::Context {
        self.inner.cx()
    }
}
