
use
{
    tokio::io::AsyncWriteExt,
    tokio::io::AsyncReadExt,
    tokio::net::TcpListener,
    std::str,
};

#[tokio::main]
pub async fn main() 
{
    //let (hosts_map,bind_addr) = handle_config().unwrap();
    let listener = TcpListener::bind(String::from("0.0.0.0:23")).await.unwrap();

    loop
    {
        let stream = listener.accept().await;

        match stream {
            Ok(mut stream) => {
                //let mut stream = stream;
                println!("new client!");
                
                let n = handle_connection(stream.0).await;

                match n
                {
                    Ok(_) => {},
                    Err(e) => println!("Connection lost {}",e),
                }
            }
            Err(e) => { /* connection failed */ }
        }
    }
}

pub async fn handle_connection(mut stream: tokio::net::TcpStream) -> Result<usize,tokio::io::Error>
{   
    println!("Connected.");
    let not_finished = true;
    stream.write_all(b"Welcome To RustTex.\r\n").await?;

    while(not_finished)
    {
        stream.write_all(b">.").await?;
        let line = read_line(&mut stream).await?;
    }

    return Ok(0);
}

pub async fn read_line(stream: &mut tokio::net::TcpStream) -> Result<String,tokio::io::Error>
{   
    let mut not_finished = true;
    let mut vec = Vec::new();

    while(not_finished)
    {
        let mut buf = [0u8;1];
        stream.read(&mut buf).await?;
        if buf[0] == 13
        {
            not_finished = false;
            stream.write_all(b"\r\n").await?;
        }
        else
        {
            vec.push(buf[0]);
            stream.write_all(&buf).await?;
        }
        println!("buf {}",buf[0]);
    }

    let x = str::from_utf8(&vec).unwrap();

    return Ok(String::from(x));
}