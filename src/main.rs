use clap::{arg, command, value_parser, Arg};
use ssh2::Session;
use std::{
    fmt::format,
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::TcpStream,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    thread,
};

fn main() -> Result<(), i32> {
    let command = command!()
        .arg(
            Arg::new("username")
                .short('u')
                .long("username")
                .value_name("USERNAME")
                .help("set the username for the ssh session")
                .required(true),
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("password")
                .value_name("PASSWORD")
                .help("set password for the ssh session")
                .required(true),
        )
        .arg(
            Arg::new("network")
                .short('n')
                .long("network")
                .value_name("NETWORK")
                .help("the ip address of the swiftbot")
                .required(true),
        )
        .arg(
            Arg::new("classpath")
                .short('c')
                .long("classpath")
                .value_name("CLASSPATH")
                .help("the classpath of the java files assuming you have the correct jars already on your swiftbot")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("entry")
                .short('e')
                .long("entry")
                .value_name("ENTRY")
                .help("entry point of program eg. org.example.Entry"),
        )
        .get_matches();

    let username = command.get_one::<String>("username").unwrap();
    let password = command.get_one::<String>("password").unwrap();
    let classpath = command.get_one::<PathBuf>("classpath").unwrap();
    let addr = command.get_one::<String>("network").unwrap();
    let entry = command.get_one::<String>("entry").unwrap();
    let full_addr = format!("{}:22", addr);

    //TODO! validate ip addr
    //
    //
    //TODO! validate classpath and entry point

    let tcp_stream = TcpStream::connect("192.168.0.100:22").unwrap_or_else(|_e| {
        println!("[INFO] error occured at initaiting TcpStream.");
        panic!("Shutting Down")
    });

    let mut session = Session::new().unwrap();

    session.set_tcp_stream(tcp_stream);
    session.handshake().unwrap();

    session.userauth_password(username, password).unwrap();
    if !session.authenticated() {
        println!("failed to authenticate try again with a diffrent password or username");
        return Err(-1);
    }
    //create a directory to store the running files and then we will delete it once were done
    let mut command_stream = session.channel_session().unwrap();

    let parts = entry.split('.').collect::<Vec<_>>();

    //setup the running directory
    command_stream = session.channel_session().unwrap();
    command_stream
        .exec(&format!(
            "mkdir -p $HOME/.running/classes/{}/{}",
            parts[0], parts[1]
        ))
        .unwrap();
    command_stream = session.channel_session().unwrap(); //send java class files idk about the jar though possibly
    println!("[INFO] looking through class paths");
    let directory = fs::read_dir(classpath).unwrap();
    let dest_path = format!("$HOME/.running/classes/{}/{}/", parts[0], parts[1]);

    for entry in directory {
        let ent = entry.unwrap();
        let name = ent.file_name().into_string().unwrap();
        let full_path = ent.path();

        let items = name.split('.').collect::<Vec<_>>();
        let file_name = items.first().unwrap().to_string();
        let extension = items.last().unwrap().to_string();
        if extension != "class" {
            continue;
        }

        let mut file = OpenOptions::new().read(true).open(full_path).unwrap();

        let size = file.metadata().unwrap().size();
        println!("[INFO] Opening java class file {}", name);

        let file_path = format!("{}.class", file_name);

        let mut remote_file = session
            .scp_send(Path::new(&file_path), 0o755, size, None)
            .unwrap();
        let mut contents = Vec::new();

        let _ = file.read_to_end(&mut contents).unwrap();
        let _ = remote_file.write(&contents).unwrap();

        command_stream
            .exec(&format!("mv {} {}", file_path, dest_path))
            .unwrap();

        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();
        command_stream = session.channel_session().unwrap();
    }
    //once thats done just run the command to run the java classpaths and check the process output
    //print it to the user and then report exit code and do some cleanup

    let exec_command = format!(
        "java -cp $HOME/.running/classes:$HOME/SwiftBot-API-5.1.3.jar {}",
        entry
    );

    println!("Please navigate via ssh to {}:22 ", addr);
    println!("and run the command {}", exec_command);
    Ok(())
}
