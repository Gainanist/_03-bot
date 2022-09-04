// use std::{
//     fs, io,
//     path::{Path, PathBuf},
//     time::Duration,
// };

// use crossbeam_channel::Receiver;
// use serde::{de::DeserializeOwned, Serialize};

// pub fn read_json<T: Default + DeserializeOwned>(path: &PathBuf) -> T {
//     match fs::read(path) {
//         Ok(games_data) => match serde_json::from_slice(&games_data) {
//             Ok(deserialized_games) => deserialized_games,
//             Err(_) => T::default(),
//         },
//         Err(_) => T::default(),
//     }
// }

// pub fn write_json_from_channel<T: Serialize, P: AsRef<Path>>(
//     receiver: &Receiver<T>,
//     path: P,
// ) -> io::Result<()> {
//     let data = receiver.recv_timeout(Duration::from_secs(1));
//     if let Ok(data) = data {
//         if let Ok(serialized) = serde_json::to_string(&data) {
//             fs::write(path, serialized)?;
//         }
//     }
//     Ok(())
// }
