
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
            Ok(stream) => {
                //let mut stream = stream;
                println!("new client!");
                
                let n = handle_connection(stream.0).await;

                match n
                {
                    Ok(_) => {},
                    Err(e) => println!("Connection lost {}",e),
                }
            }
            Err(_e) => { /* connection failed */ }
        }
    }
}

pub async fn handle_connection(mut stream: tokio::net::TcpStream) -> Result<usize,tokio::io::Error>
{   
    println!("Connected.");
    let mut not_finished = true;
    stream.write_all(b"\x0c\x1E").await?; //Welcome To RustTex.\r\n").await?;

    load_page_to_stream(&mut stream,"title.tti",-1).await?;
    stream.write_all(b"\r\n").await?;
    while not_finished
    {
        stream.write_all(b">.").await?;
        let line = read_line(&mut stream).await?.to_lowercase();

        if line == "quit"
        {
            not_finished = false;
        }
        else if line == "help"
        {
            stream.write_all(b"\x0c\x1E").await?;
            load_page_to_stream(&mut stream,"help.tti",-1).await?;
        }
        else if line == "menu"
        {
            stream.write_all(b"\x0c\x1E").await?; //Welcome To RustTex.\r\n").await?;

            load_page_to_stream(&mut stream,"title.tti",-1).await?;
            stream.write_all(b"\r\n").await?;
        }
        else if line == "http"
        {
            stream.write_all(b"ADDR:").await?;
            let url = read_line(&mut stream).await?.to_lowercase();
            load_page_from_addr(&mut stream, &url).await?;

            stream.write_all(b"\r\n").await?;
        }
        else if line == "cls"
        {
            stream.write_all(b"\x0c").await?;
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

// This translates the escape codes to create text data which is understood by the BBC micro.
// It will return Ok(1) if double height characters are used.
pub async fn de_escape(stream: &mut tokio::net::TcpStream, buf:&[u8]) -> Result<u8,std::io::Error>
{
    let mut repeat = 0;

    println!("{}", buf.hex_dump());

    let mut prev = 0;
    for i in buf
    {
        // Strip out carriage returns and line feeds as they shouldn't be within the line data 
        // and we add our own.
        if *i != b'\r' || *i != b'\n'
        {
            if prev == 0x1B
            {
                if *i == 0x4d
                {
                    repeat = 1;
                }
                println!("Esc:{}",*i-(0x40 as u8));
                stream.write_u8(*i+(0x40 as u8)).await?;
            }
            else if *i != 0x1B 
            {
                let mut ch = *i;
                if *i > 0x20 && *i < 0x80
                {
                    ch = *i + 128;
                }
                stream.write_u8(ch).await?;
            }
        prev = *i;
        }
    }

    return Ok(repeat);
}


// This translates the escape codes to create text data which is understood by the BBC micro.
// It will return Ok(1) if double height characters are used.
pub async fn de_escape_mode7_utf(stream: &mut tokio::net::TcpStream, buf:&[u8], line_b:bool) -> Result<u8,std::io::Error>
{
    let mut repeat = 0;

    println!("{}", buf.hex_dump());
    let mut foreground_colour = 6;
    let mut double_height = false;
    let mut graphics = false;
    let mut seperated = false;
    let mut prev = 0;
    for i in buf
    {
        // Strip out carriage returns and line feeds as they shouldn't be within the line data 
        // and we add our own.
        if *i != b'\r' || *i != b'\n'
        {
            let mut ch = None;
            if prev == 0x1B
            {
                if *i == 0x4d
                {
                    repeat = 1;
                }
                println!("Esc:{}",*i-(0x40 as u8));

                ch = Some(*i+(0x40 as u8));


            }
            else if *i != 0x1B 
            {
                ch = Some(*i);
            }
        
        if let Some(mut ch) = ch 
        {
            print!("ch:{} ",ch);
            if ch >= 145 && ch <= 151
            {
                graphics = true;
                foreground_colour = ch - 145;
                stream.write_all("\u{001b}[".as_bytes()).await?;
                
                let colour = String::from((ch-145+31).to_string()+";1m ");
                stream.write_all(colour.as_bytes()).await?;
            } 
            else if ch >= 129 && ch <= 135
            {   
                graphics = false; 
                foreground_colour = ch - 129;
                stream.write_all("\u{001b}[".as_bytes()).await?;

                let colour = String::from((ch-129+31).to_string()+";1m ");
                stream.write_all(colour.as_bytes()).await?;
            }
            else if ch == 157
            { 
                stream.write_all("\u{001b}[".as_bytes()).await?;

                let colour = String::from((foreground_colour+101).to_string()+";1m ");
                stream.write_all(colour.as_bytes()).await?;

            }
            else if ch == 156
            { 
                stream.write_all("\u{001b}[40;1m".as_bytes()).await?;
            }
            else
            {   
                let mut base_code = 0xe000;
                if ch == 141
                {
                    double_height = true;
                    ch = b' ';
                }
                else if ch == 140
                {
                    double_height = false;
                    ch = b' ';
                }
                if graphics && ch >= 95 && ch <= 127 
                {
                    ch = ch + 128;
                }
                
                if graphics && ch > 160
                {
                    let mut buf = [0;3];
                    let x = char::from_u32(((ch-160) as u32) + base_code + 0x200);
                    
                    let x = x.unwrap();

                    let result = x.encode_utf8(&mut buf);

                    stream.write(&buf).await?;
                }
                else if graphics==true && ch > 32 && ch < 63 //&& ch != 35
                {
                    let mut buf = [0;3];
                    let x = char::from_u32((ch as u32) + base_code + 0x200 - 32);
                    
                    let x = x.unwrap();

                    x.encode_utf8(&mut buf);

                    stream.write(&buf).await?;
                }
                else if double_height
                {
                    let mut shift = 0xe000;
                    if line_b
                    {
                        shift = 0xe100;
                    }
                    let mut buf = [0;3];
                    let x = char::from_u32((ch as u32) + shift);
                    let x = x.unwrap();

                    x.encode_utf8(&mut buf);

                    stream.write(&buf).await?;
                }
                else
                {
                    stream.write_u8(ch).await?;
                }
            }
        }
        prev = *i;
        }
    }
    stream.write_all("\u{001b}[37;1m".as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    return Ok(repeat);
}


pub async fn load_page_from_addr(stream_out: &mut tokio::net::TcpStream,url_str: &str) -> Result<u8,std::io::Error>
{
    use hyper::Client;
    let client = Client::new();
    let uri_res = url_str.parse();
    println!("URL:{}",url_str);

    match uri_res 
    {
        Ok(uri) => {
            let resp_r = client.get(uri).await;

            if let Ok(resp) = resp_r
            {
                println!("Response: {}", resp.status());

                let buf_r = hyper::body::to_bytes(resp.into_body()).await;

                if let Ok(buf) = buf_r
                {
                    stream_out.write_all(b"\x0c").await?;
                    render_page_to_stream(stream_out,&buf, -1).await?;
                }
                
            }
        },
        Err(_e) => {stream_out.write_all(b"Couldn't load page.").await?;}
    }

    return Ok(0);
}



pub async fn load_page_to_stream(stream: &mut tokio::net::TcpStream,filename: &str, page_no:i32) -> Result<i32,std::io::Error>
{
   
    let mut buf = Vec::new();
    let mut file = std::fs::File::open(filename)?;
    
    let x = file.read_to_end(&mut buf).unwrap();

    
    println!("Buf:{}",x);
    //std::io::Write::write_all(&mut std::io::stdout(),&buf);

    return render_page_to_stream(stream,&buf, page_no).await
}

// Renders the page TTI file stored in buf to stream.  
// If request_page_no is set to -1 then it will render the first page found otherwise it will render the page requested.println!
// returns 
// Ok(page_no) = Page no of the page found.
// 
pub async fn render_page_to_stream(stream: &mut tokio::net::TcpStream,buf: &[u8], requested_page_no:i32) -> Result<i32,std::io::Error>
{
    let mut page_no = -1; // When this is -1 it indicates to the render engine that no page has been found.

    let mut x = 0;
    let mut y = 0; // y always points to the start of the current line being processed.
    let mut prev_ol = 0;
    let mut cur_ol = 0;
    let mut arg_no = 0;

    let mut command = None;
    let mut line = None;

    // This loop splits the page into lines and extracts the entries marked OL,<Line-no> where line-no > 0.
    for i in buf 
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
                println!("LINE:{}",str::from_utf8(v).unwrap());
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
                { 
                    if s == "OL" 
                    {
                        if page_no == requested_page_no || requested_page_no == -1
                        {
                            print = true;
                        }
                    }
                    if s == "PN"
                    {
                        if page_no >=0 && requested_page_no == -1
                        {
                            // We wanted only one page and we've found another one so there's no point scanning the rest of the buffer.
                            break;
                        }
                        let page_no_r = i32::from_str_radix(str::from_utf8(&buf[y..x-1]).unwrap(), 16);

                        if let Ok(page_no_found) = page_no_r 
                        {
                            page_no = page_no_found;
                        }

                        println!("Page no:{}",str::from_utf8(&buf[y..x-1]).unwrap());
                    }
                }
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
                            Err(_e) => {} 
                        }
                    }
                }
                None => {print=false;}
            } 
            if print
            {
                //std::io::Write::write_all(&mut std::io::stdout(),&buf[y..x-1]).unwrap();
                println!("{},{}:",cur_ol,prev_ol);

                if prev_ol+1 != cur_ol
                {
                    stream.write(b"\r\n").await?;
                }
                if de_escape_mode7_utf(stream,&buf[y..x-1],false).await? == 1
                {
                    de_escape_mode7_utf(stream,&buf[y..x-1],true).await?;
                    cur_ol = cur_ol + 1;
                }
                //stream.write(&buf[y..x-1]).await?;
                //stream.write(b"\r\n").await?;
            }

            prev_ol = cur_ol;
            y = x + 1; // The next value in the buffer will be the start of the next line so record that.
            arg_no = 0;
            line = None;
            command = None;
        }
        x = x + 1;
    }
    return Ok(page_no);
}