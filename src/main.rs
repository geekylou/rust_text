
use
{
    tokio::io::AsyncWriteExt,
    tokio::io::AsyncReadExt,
    tokio::net::TcpListener,
    std::str,
    std::io::Seek,
    std::fs::File,
    std::io::Read,
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
    let mut not_finished = true;
    stream.write_all(b"\x0c\x1E").await?; //Welcome To RustTex.\r\n").await?;

    load_page_to_stream(&mut stream,"title.tti",100).await?;
    stream.write_all(b"\r\n").await?;
    while not_finished
    {
        stream.write_all(b">.").await?;
        let line = read_line(&mut stream).await?;

        if line == "quit"
        {
            not_finished = false;
        }
        println!("Line:{}",line);
    }
    return Ok(0);
}

pub async fn read_line(stream: &mut tokio::net::TcpStream) -> Result<String,tokio::io::Error>
{   
    let mut not_finished = true;
    let mut vec = Vec::new();

    while not_finished
    {
        let mut buf = [0u8;1];
        stream.read(&mut buf).await?;

        println!("buf:{}",buf[0]);

        match buf[0] 
        {
            13 => // Enter
            {
                not_finished = false;
                stream.write_all(b"\r\n").await?;
            }
            127 => // del
            {
                if vec.pop() != None
                {
                    stream.write(&buf).await?;
                }
            }
            _ => 
            {
                vec.push(buf[0]);
                stream.write_all(&buf).await?;
            }
        }
    }

    let x = str::from_utf8(&vec).unwrap();
    return Ok(String::from(x));
}

use pretty_hex::*;

pub async fn de_escape(stream: &mut tokio::net::TcpStream, buf:&[u8]) -> Result<u8,std::io::Error>
{


    println!("{}", buf.hex_dump());

    let mut prev = 0;
    for i in buf
    {
        if prev == 0x1B
        {
            println!("Esc:{}",*i-(0x40 as u8));
            stream.write_u8(*i+(0x40 as u8)).await?;
        }
        else if *i != 0x1B 
        {
            let mut ch = *i;
            if (*i > 0x20)
            {
                ch = *i + 128;
            }
            stream.write_u8(ch).await?;
        }
        prev = *i;
    }

    return Ok(0);
}

pub async fn load_page_to_stream(stream: &mut tokio::net::TcpStream,filename: &str, page_no:u8) -> Result<String,std::io::Error>
{
   
    let mut buf = Vec::new();
    let mut file = std::fs::File::open(filename)?;
    
    let x = file.read_to_end(&mut buf).unwrap();

    
    println!("Buf:{}",x);
    //std::io::Write::write_all(&mut std::io::stdout(),&buf);

    let mut x = 0;
    let mut y = 0;
    let mut prev_ol = 0;
    let mut cur_ol = 0;
    let mut arg_no = 0;

    let mut command = None;
    let mut line = None;
    
    for i in &buf 
    {
        if *i == b','
        {
            let v = &buf[y..x];

            if arg_no == 0
            {
                command = Some(str::from_utf8(v).unwrap());
                println!("COMMAND:{}",str::from_utf8(v).unwrap());                
            }
            if arg_no == 1
            {
                line = Some(str::from_utf8(v).unwrap());
            }
            //println!("sep:{}",x);
            //std::io::Write::write_all(&mut std::io::stdout(),&v).unwrap();

            arg_no = arg_no + 1;
            y = x + 1;
        }
        if *i == b'\n'
        {
            let mut print = false;
            match command
            {
                Some(s) => 
                { if s == "OL" 
                    {
                        print = true;
                    }}
                None => {print = false;}
            }
            match line 
            {
                Some(s) =>
                {
                    if s == "0"
                    {
                        print = false;
                    }
                    else
                    {
                        match s.parse::<i32>()
                        {
                            Ok(ol) => cur_ol = ol,
                            Err(e) => {} 
                        }
                    }
                }
                None => {print=false;}
            } 
            if print
            {
                std::io::Write::write_all(&mut std::io::stdout(),&buf[y..x-1]).unwrap();
                println!("{},{}:",cur_ol,prev_ol);

                if (prev_ol+1 != cur_ol)
                {
                    stream.write(b"\r\n").await?;
                }
                de_escape(stream,&buf[y..x-2]).await?;
                //stream.write(&buf[y..x-1]).await?;
                stream.write(b"\r\n").await?;
            }

            prev_ol = cur_ol;
            y = x + 1;
            //println!("x:{} y:{}",x,y);
            arg_no = 0;
            line = None;
            command = None;
        }
        x = x + 1;
    }
    return Ok(String::from(""));
}