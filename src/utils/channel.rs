use bevy::utils::HashMap;
use crossbeam_channel::Receiver;

pub fn if_channels_has_message<T>(channels: &HashMap<usize, Receiver<T>>) -> bool
{
    for channel in channels.values()
    {
        if !channel.is_empty()
        {
            return true;
        }
    }
    false
}
