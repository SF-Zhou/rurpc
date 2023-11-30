use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use byteorder::ByteOrder;
use bytes::{Buf, BufMut};
use pilota::prost::Message;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct S;
impl proto_gen::example::EchoService for S {
    async fn echo(
        &self,
        req: &proto_gen::example::EchoRequest,
    ) -> std::result::Result<proto_gen::example::EchoResponse, std::boxed::Box<dyn std::error::Error>>
    {
        let mut resp = proto_gen::example::EchoResponse::default();
        resp.message = req.message.clone();
        Ok(resp)
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:8000").await?;

    let qps = Arc::new(AtomicU64::new(0));
    let reader = qps.clone();
    tokio::spawn(async move {
        let qps: &AtomicU64 = reader.as_ref();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            println!("QPS: {}/s", qps.swap(0u64, Ordering::SeqCst));
        }
    });

    loop {
        // The second item contains the IP and port of the new connection.
        if let Ok((socket, addr)) = listener.accept().await {
            println!("incoming {}", addr);
            let writer = qps.clone();
            tokio::spawn(async move {
                let _ = echo(socket, writer).await;
                println!("closed {}", addr);
            });
        }
    }
}

async fn echo(
    mut socket: TcpStream,
    qps: Arc<AtomicU64>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    const BAIDU_MAGIC_NUMBER: u32 = 1347571779u32;
    const BAIDU_HEADER_SIZE: usize = 12;
    let mut header = [0u8; BAIDU_HEADER_SIZE];
    let qps: &AtomicU64 = qps.as_ref();
    let server = proto_gen::example::EchoServiceServer::new(S);
    loop {
        socket.read_exact(&mut header).await?;
        let mut slice = &header[..];
        let magic_number = byteorder::BigEndian::read_u32(&mut slice);
        if magic_number != BAIDU_MAGIC_NUMBER {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid header",
            )));
        }
        slice.advance(std::mem::size_of::<u32>());
        let body_length = byteorder::BigEndian::read_u32(&mut slice) as usize;
        slice.advance(std::mem::size_of::<u32>());
        let meta_length = byteorder::BigEndian::read_u32(&mut slice) as usize;

        let mut buffer = bytes::BytesMut::with_capacity(body_length);
        unsafe { buffer.set_len(body_length) };
        let _ = socket.read_exact(&mut buffer).await?;

        let meta_slice = &buffer[..meta_length];
        let mut meta = baidu_rpc_meta::brpc::policy::RpcMeta::decode(meta_slice)?;
        let msg_slice = &buffer[meta_length..];

        let req_meta = meta.request.unwrap();
        let method = format!("/{}/{}", req_meta.service_name, req_meta.method_name);
        let mut response_meta = baidu_rpc_meta::brpc::policy::RpcResponseMeta::default();
        let rsp_buffer = match server.call(&method, msg_slice).await {
            Ok(rsp) => rsp,
            Err(e) => {
                response_meta.error_code = Some(-1);
                response_meta.error_text = Some(e.to_string().into());
                bytes::Bytes::new()
            }
        };
        meta.request = None;
        meta.response = Some(response_meta);
        let meta_length = meta.encoded_len() as u32;
        let mut header_buffer =
            bytes::BytesMut::with_capacity(meta_length as usize + BAIDU_HEADER_SIZE);
        let mut meta_buffer = header_buffer.split_off(BAIDU_HEADER_SIZE);
        meta.encode(&mut meta_buffer)?;
        let rsp_length = rsp_buffer.len() as u32;
        let body_length = meta_length + rsp_length;
        header_buffer.put_u32(magic_number);
        header_buffer.put_u32(body_length);
        header_buffer.put_u32(meta_length);
        header_buffer.unsplit(meta_buffer);

        socket.write_all(&header_buffer).await?;
        socket.write_all(&rsp_buffer).await?;
        qps.fetch_add(1, Ordering::SeqCst);
    }
}
