use crate::{
    model::Stone,
    session
};
use tokio::{
    io::{BufReader, AsyncBufReadExt, AsyncWriteExt},
    process::Command,
    sync::mpsc
};
use std::process::Stdio;

pub enum Message {
    Play(Stone, u32),
    Genmove,
    Handicap(u32),
    Quit
}

fn index_to_vertex(index: u32, board_size: u32) -> String {
    let x = index % board_size;
    let y = index / board_size;
    let col = unsafe {
        char::from_u32_unchecked(
            if x <= 7 {
                'A' as u32 + x
            } else {
                'A' as u32 + x + 1
            }
        )
    };
    let row = board_size - y;
    col.to_string() + &row.to_string()
}

fn vertex_to_index(vertex: &str, board_size: u32) -> Option<u32> {
    let c: char = vertex.chars().next().unwrap();
    let x = if c < 'I' {
        c as u32 - 'A' as u32
    } else {
        c as u32 - 'A' as u32 - 1
    };
    if let Ok(row) = vertex[1..].parse::<u32>() {
        let y = board_size - row;
        Some(board_size * y + x)
    } else {
        None
    }
}

pub fn engine(
    command: String,
    stone: Stone,
    board_size: u32,
    komi: u32,
    sender: mpsc::UnboundedSender<session::Message>
) -> mpsc::UnboundedSender<Message> {
    let mut child = Command::new(&command)
        .args(["--mode", "gtp", "--level", "1", "--chinese-rules", "--allow-suicide"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error spawning child");
    //Read from stdout
    let stdout = child.stdout.take().unwrap();
    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let line = line.trim_end();
            //println!("ENGINE: {}", line);
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() == 2 {
                //= [vertex/pass/resign]
                let part = parts[1].to_uppercase();
                match part.as_str() {
                    "PASS" => if sender.send(session::Message::Pass(stone)).is_err() {
                        break
                    },
                    "RESIGN" => if sender.send(session::Message::Resign(stone)).is_err() {
                        break
                    },
                    _ => {
                        let message = if let Some(index) = vertex_to_index(&part, board_size) {
                            session::Message::Play(stone, index as usize)
                        } else {
                            session::Message::Resign(stone)
                        };
                        if sender.send(message).is_err() {
                            break
                        }
                    }
                }
            } else if parts.len() > 2 {
                //= vertex vertex...
                let positions: Vec<usize> = parts[1..].iter().map(
                    |x| vertex_to_index(x, board_size).unwrap() as usize
                ).collect();
                if sender.send(session::Message::Handicap(stone, positions)).is_err() {
                    break
                }
            }
        }
    });
    //Write to stdin
    let mut stdin = child.stdin.take().unwrap();
    let (sender, mut receiver) = mpsc::unbounded_channel::<Message>();
    tokio::spawn(async move {
        let _ = stdin.write_all(format!("boardsize {}\n", board_size).as_bytes()).await;
        let _ = stdin.write_all(format!("komi {}\n", komi).as_bytes()).await;
        //Listen for messages
        while let Some(message) = receiver.recv().await {
            match message {
                Message::Handicap(count) => {
                    if stdin.write_all(format!(
                        "place_free_handicap {}\n", count
                    ).as_bytes()).await.is_err() {
                        break
                    }
                },
                Message::Play(stone, position) => {
                    let stone: &str = stone.into();
                    let vertex = index_to_vertex(position, board_size);
                    if stdin.write_all(format!(
                        "play {} {}\n", stone, vertex
                    ).as_bytes()).await.is_err() {
                        break
                    }
                },
                Message::Genmove => {
                    let stone: &str = stone.into();
                    if stdin.write_all(format!(
                        "genmove {}\n", stone
                    ).as_bytes()).await.is_err() {
                        break
                    }
                },
                Message::Quit => {
                    if stdin.write_all(b"quit\n").await.is_err() {
                        break
                    } else {
                        break
                    }
                }
            }
        }
        handle.abort(); //Stop stdout reader
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::{index_to_vertex, vertex_to_index};
    #[test]
    fn test_index_to_vertex() {
        //Corners
        assert_eq!(index_to_vertex(0, 19), "A19");
        assert_eq!(index_to_vertex(18, 19), "T19");
        assert_eq!(index_to_vertex(342, 19), "A1");
        assert_eq!(index_to_vertex(360, 19), "T1");
        //Edges
        assert_eq!(index_to_vertex(8, 19), "J19");
        //Center
        assert_eq!(index_to_vertex(180, 19), "K10");
    }
    #[test]
    fn test_vertex_to_index() {
        //Corners
        assert_eq!(vertex_to_index("A19", 19), Some(0));
        assert_eq!(vertex_to_index("T19", 19), Some(18));
        assert_eq!(vertex_to_index("A1", 19), Some(342));
        assert_eq!(vertex_to_index("T1", 19), Some(360));
        //Center
        assert_eq!(vertex_to_index("K10", 19), Some(180));
    }
}
