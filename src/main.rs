
use
{
    tokio::io::AsyncWriteExt,
    tokio::io::AsyncReadExt,
    tokio::net::TcpListener,
    std::str,
    std::io::Read,
    async_trait::async_trait,
    std::path::Path,
};

use std::collections::HashMap;

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
                tokio::spawn(async move 
                    {
                    let mut n = Ok(0);
                    {
                        let mut stream = stream.0;
                        loop
                        { 
                            let q = stream.write_all(b"Are you running on BBC Micro(b) or UTF-8 terminal emulator (U)?").await;
                            match q
                            {
                                Ok(_) => {},
                                Err(_e) => break,
                            }
                            if let Ok(x) = read_key(&mut stream).await
                            {
             
                                if x=='b'
                                {
                                    n = handle_connection(stream,&Mode7BeebAscii).await;
                                    break;
                                }
                                else if x=='u'
                                {
                                    n = handle_connection(stream,&Mode7UTF8Ansi).await;
                                    break;
                                }
                            }
                        }
                    }
                    match n
                    {
                        Ok(_) => {},
                        Err(e) => println!("Connection lost {}",e),
                    }
                });
            }
            Err(_e) => { /* connection failed */ }
        }
    }
}

async fn handle_connection(mut stream: tokio::net::TcpStream,decoder:&impl TTIDecoder) -> Result<usize,tokio::io::Error>
{   
    let mut page_stack = Vec::new();
    let mut nav = None;
    println!("Connected.");
    let mut not_finished = true;

    stream.write_all(b"\x0c\x1E").await?; //Welcome To RustTex.\r\n").await?;

    load_page_to_stream(&mut stream,"title.tti",-1,decoder).await?;
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
            load_page_to_stream(&mut stream,"help.tti",-1,decoder).await?;
        }
        else if line == "menu"
        {
            stream.write_all(b"\x0c\x1E").await?; //Welcome To RustTex.\r\n").await?;

            load_page_to_stream(&mut stream,"title.tti",-1,decoder).await?;
            stream.write_all(b"\r\n").await?;
        }
        else if line == "http"
        {
            stream.write_all(b"ADDR:").await?;
            let url = read_line(&mut stream).await?.to_lowercase();
            nav = load_page_from_addr(&mut stream, &url,decoder).await?;
            if let Some(nav) = &nav
            {
                if let Some(uri) = nav.uri.clone()
                {
                    page_stack.push(uri);
                }
            }
            stream.write_all(b"\r\n").await?;
        }
        else if line == "cls"
        {
            stream.write_all(b"\x0c").await?;
        }
        else if line == "back"
        {
            if let Some(prev_uri) = page_stack.pop()
            {
                nav = load_page_from_uri(&mut stream, prev_uri,decoder).await?;
            } 
        }
        else if line == "reload"
        {
            println!("try to reload");
            if let Some(nav_r) = &nav
            {
                if let Some(uri) = nav_r.uri.clone()
                {
                    println!("Reload: {}",uri);
                    nav = load_page_from_uri(&mut stream, uri,decoder).await?;
                }
            }
        }
        else
        {           
            let nav_t = select_link(&mut stream, &line,&nav,decoder).await?;
            // We don't want to replace nav unless it not-null otherwise we won't be able to reload the current page!
            if let Some(nav_r) = &nav
            {
                if let Some(uri) = nav_r.uri.clone()
                {
                    page_stack.push(uri);
                }
                nav = nav_t;
            }
        }
        println!("Line:{}",line);
    }
    return Ok(0);
}

async fn select_link(stream_out: &mut tokio::net::TcpStream,line: &str, navigation:&Option<Navigation>,decoder: &impl TTIDecoder) -> Result<Option<Navigation>,std::io::Error>
{
    if let Some(nav_r) = navigation
    {
        if let Some(link) = nav_r.links.get(line)
        {
            if  let Some(uri) = &nav_r.uri
            {
                if let Some(path) = Path::new(uri.path()).parent()
                {
                    use hyper::http::Uri;
                    let mut uri_builder = Uri::builder();
                    
                    if let Some(scheme) = uri.scheme()
                    {
                        uri_builder = uri_builder.scheme(scheme.clone());
                    }
                    if let Some(authority) = uri.host()
                    {
                        uri_builder = uri_builder.authority(authority.clone());
                    }
                    
                    if let Ok(uri) = uri_builder.path_and_query(String::from(path.to_str().unwrap())+&String::from("/")+link).build()
                    {
                        println!("Uri:{}",uri);
                        return load_page_from_uri(stream_out, uri,decoder).await;
                    }

                    println!("Goto url:{} {}",link, path.to_str().unwrap());
                }
            }
        }
    }
    return Ok(None);
}

// TODO: Fix error handling support UTF-8 properly.
pub async fn read_key(stream: &mut tokio::net::TcpStream) -> Result<char,tokio::io::Error>
{
    let mut buf = [0u8;1];
    stream.read(&mut buf).await?;

    if let Some(x) = char::from_u32(buf[0] as u32)
    {
        return Ok(x);
    }
    return Ok(' ');

}

pub async fn read_line(stream: &mut tokio::net::TcpStream) -> Result<String,tokio::io::Error>
{   
    let mut not_finished = true;
    let mut vec = Vec::new();

    while not_finished
    {
        let mut buf = [0u8;1];
        stream.read(&mut buf).await?;

        //println!("buf:{}",buf[0]);

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

struct Navigation
{
    uri:Option<hyper::Uri>,
    start_page:i32,
    links:HashMap<String,String>,
}

struct Mode7BeebAscii;
struct Mode7UTF8Ansi;

#[async_trait]
trait TTIDecoder 
{
    // Static method signature; `Self` refers to the implementor type.
    async fn de_escape(&self,stream: &mut tokio::net::TcpStream, buf:&[u8], line_b:bool) -> Result<u8,std::io::Error>;
}

impl Mode7BeebAscii
{
    fn new() -> Mode7BeebAscii
    {
        Mode7BeebAscii{}
    }
}
#[async_trait]
impl TTIDecoder for Mode7BeebAscii
{
    // This translates the escape codes to create text data which is understood by the BBC micro.
    // It will return Ok(1) if double height characters are used.
    async fn de_escape(&self,stream: &mut tokio::net::TcpStream, buf:&[u8],_line_b:bool) -> Result<u8,std::io::Error>
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
}

impl Mode7UTF8Ansi
{
    fn new() -> Mode7UTF8Ansi
    {
        Mode7UTF8Ansi{}
    }
}
#[async_trait]
impl TTIDecoder for Mode7UTF8Ansi
{

    // This translates the escape codes to create text data which is understood by the BBC micro.
    // It will return Ok(1) if double height characters are used.
    async fn de_escape(&self,stream: &mut tokio::net::TcpStream, buf:&[u8], line_b:bool) -> Result<u8,std::io::Error>
    {
        let mut repeat = 0;

        //println!("{}", buf.hex_dump());
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
                    /*if *i == 0x4d
                    {
                        repeat = 1;
                    }*/
                    //println!("Esc:{}",*i-(0x40 as u8));

                    ch = Some(*i+(0x40 as u8));
                }
                else if *i != 0x1B 
                {
                    ch = Some(*i);
                }
            
            if let Some(mut ch) = ch 
            {
                //print!("ch:{} ",ch);
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

                    if seperated
                    {
                        base_code = 0xe0c0;
                    }
                    if double_height
                    {
                        if line_b
                        {
                            base_code = base_code + 0x80
                        }
                        else
                        {
                            base_code = base_code + 0x40
                        }
                    }

                    if ch == 153 // Turn off seperated mode.
                    {
                        seperated = false;
                        ch = b' ';
                    }
                    if ch == 154 // Turn on seperated mode.
                    {
                        seperated = true;
                        ch = b' ';
                    }
                    if ch == 141
                    {
                        repeat = 1;
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
                    
                    if graphics && ch > 160 && ch <= 191
                    {
                        let mut buf = [0;3];
                        let x = char::from_u32(((ch-160) as u32) + base_code + 0x200);
                        
                        let x = x.unwrap();

                        x.encode_utf8(&mut buf);

                        stream.write(&buf).await?;
                    }
                    if graphics && ch >= 224 
                    {
                        let mut buf = [0;3];
                        let x = char::from_u32(((ch-192) as u32) + base_code + 0x200);
                        
                        let x = x.unwrap();

                        x.encode_utf8(&mut buf);

                        stream.write(&buf).await?;
                    }
                    else if graphics==true && ch > 32 && ch <= 63 //&& ch != 35
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
        stream.write_all("\u{001b}[37;1m\u{001b}[40m".as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
        return Ok(repeat);
    }
}
async fn load_page_from_addr(stream_out: &mut tokio::net::TcpStream,url_str: &str,decoder: &impl TTIDecoder) -> Result<Option<Navigation>,std::io::Error>
{
    let uri_res = url_str.parse::<hyper::Uri>();
    println!("URL:{}",url_str);

    match uri_res 
    {
        Ok(uri) => {
            return load_page_from_uri(stream_out,uri,decoder).await;
        }

        Err(_e) => {stream_out.write_all(b"Couldn't load page. Unable to parse URI.").await?;}
    }
    return Ok(None);
}

async fn load_page_from_uri(stream_out: &mut tokio::net::TcpStream,uri: hyper::Uri,decoder: &impl TTIDecoder) -> Result<Option<Navigation>,std::io::Error>
{
    use hyper::Client;
    let client = Client::new();
    let resp_r = client.get(uri.clone()).await;

    if let Ok(resp) = resp_r
    {
        println!("Response: {}", resp.status());

        let status = resp.status();
        let buf_r = hyper::body::to_bytes(resp.into_body()).await;

        if let Ok(buf) = buf_r 
        {
            if status.is_success()
            {
                stream_out.write_all(b"\x0c").await?;
                let res = render_page_to_stream(stream_out,&buf, -1,decoder).await?;

                if let Some(mut nav_r) = res
                {
                    println!("Add uri:{}",uri);
                    nav_r.uri = Some(uri);
                    return Ok(Some(nav_r));
                }
                
                return Ok(res);
            }
            else
            {   
                stream_out.write_all(b"Could not load page:").await?;
                stream_out.write_all(status.as_str().as_bytes()).await?;
                stream_out.write_all(b"\r\n").await?;
            } 
        }
        
    }
    else if let Err(e) = resp_r
    {
        stream_out.write_all(b"Could not load page:").await?;
        stream_out.write_all(e.to_string().as_bytes()).await?;
        stream_out.write_all(b"\r\n").await?;
    }

    return Ok(None);
}

async fn load_page_to_stream(stream: &mut tokio::net::TcpStream,filename: &str, page_no:i32,decoder: &impl TTIDecoder) -> Result<Option<Navigation>,std::io::Error>
{ 
    let mut buf = Vec::new();
    let mut file = std::fs::File::open(filename)?;
    
    let x = file.read_to_end(&mut buf).unwrap();
    
    println!("Buf:{}",x);
    //std::io::Write::write_all(&mut std::io::stdout(),&buf);

    return render_page_to_stream(stream,&buf, page_no,decoder).await
}

// Renders the page TTI file stored in buf to stream.  
// If request_page_no is set to -1 then it will render the first page found otherwise it will render the page requested.println!
// returns 
// Ok(page_no) = Page no of the page found.
// 
async fn render_page_to_stream(stream: &mut tokio::net::TcpStream,buf: &[u8], requested_page_no:i32,decoder: &impl TTIDecoder) -> Result<Option<Navigation>,std::io::Error>
{
    let mut navigation = Navigation { links: HashMap::new(), start_page: -1, uri:None };
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
                //println!("COMMAND:{}",str::from_utf8(v).unwrap());                
            }
            if arg_no == 1
            {
                //println!("LINE:{}",str::from_utf8(v).unwrap());
                line = Some(str::from_utf8(v).unwrap());
            }

            //println!("sep:{}",x);
            //std::io::Write::write_all(&mut std::io::stdout(),&v).unwrap();
            // [TODO] Fix
            if arg_no < 2
            {
                arg_no = arg_no + 1;
                y = x + 1;
            }
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

                        //println!("Page no:{}",str::from_utf8(&buf[y..x-1]).unwrap());
                    }
                    if s == "LN"
                    {
                        if let Ok(st) = str::from_utf8(&buf[y..x-1])
                        {
                            if let Some(line_r) = line
                            {
                                navigation.links.insert(String::from(line_r),String::from(st));
                            }
                        }
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
                //println!("{},{}:",cur_ol,prev_ol);

                if prev_ol+1 != cur_ol
                {
                    stream.write(b"\r\n").await?;
                }
                if decoder.de_escape(stream,&buf[y..x-1],false).await? == 1
                {
                    decoder.de_escape(stream,&buf[y..x-1],true).await?;
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

    navigation.start_page = page_no;
    return Ok(Some(navigation));
}