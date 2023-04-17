use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
use std::thread;
use std::time;
use mkultra;

//TODO: Comment this code!
pub struct Chat {
    pub id: String
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let args: Vec<String> = env::args().collect();
    let arg_port = &args[1];

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let bind_addr = format!("0.0.0.0:{}", arg_port);
    let addr = bind_addr.clone();
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");


    //Make a keep-alive thread
    let peer_map = state.clone();
    thread::spawn(move || {
        loop {
            println!("Sending keep-alives...");

            // We want to broadcast the message to everyone except ourselves.
            let peers = peer_map.lock().unwrap();
            let broadcast_recipients =
                peers.iter().filter(|(peer_addr, _)| peer_addr == peer_addr).map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                println!("  ...to Peer: {:#?}", recp);
                let result = recp.unbounded_send(Message::Text("{}".to_string()));
                match result {
                    Ok(_) => {
                        println!("  Ping sent.");
                    }
                    Err(e) => {
                        eprintln!("  Error [unbounded_send]: {}", e);
                    }
                }
            }

            std::mem::drop(peers);

            thread::sleep(time::Duration::from_millis(47000));
        }
    });

    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}


async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    let guard = peer_map.lock().unwrap().insert(addr, tx);
    std::mem::drop(guard);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let msg_in = msg.to_text().unwrap();
        println!("Received a message from {}", addr);
        let peers = peer_map.lock().unwrap();

        //Add a comment if it's not blank
        let json_result = serde_json::from_str(msg_in);
        match json_result {
            Ok(comment) => {
                let mut comment: mkultra::Comment = comment;
                let msg_id: u64 = comment.id;
                let chat_id: String = comment.chat_id.clone();
                if !comment.comment.trim().is_empty() && !comment.comment.contains("<script>")  {
                    comment.id = 0;
                    match mkultra::add_comment_to_db(comment) {
                        Ok(_) => {
                            println!("Comment added.\n");
                        }
                        Err(e) => {
                            eprintln!("Error adding comment: {}", e);
                        }
                    }
                }

                //Get the comments
                let comments = mkultra::get_comments_by_chat_id(&chat_id, msg_id).unwrap();
                let json_comments = serde_json::to_string(&comments).unwrap();

                // We want to broadcast the message to everyone except ourselves.
                let broadcast_recipients =
                    peers.iter().filter(|(peer_addr, _)| peer_addr == peer_addr).map(|(_, ws_sink)| ws_sink);

                for recp in broadcast_recipients {
                    let result = recp.unbounded_send( Message::Text(json_comments.clone().into()) );
                    match result {
                        Ok(_) => {
                            println!("  Message sent.");
                        }
                        Err(e) => {
                            eprintln!("  Error [unbounded_send]: {}", e);
                        }
                    }
                }
            }
            Err (e) => {
                eprintln!("  Error unwrapping json: {}", e);
            }
        }


        std::mem::drop(peers);

        future::ok(())
    });


    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    let guard = peer_map.lock().unwrap().remove(&addr);
    std::mem::drop(guard);
}