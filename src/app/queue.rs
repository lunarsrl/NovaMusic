// SPDX-License-Identifier: GPL-2.0-or-later
//! Track queue storage and playback ordering (linear or shuffled).

use crate::app::AppTrack;
use rand::seq::SliceRandom;

/// Whether shuffle is enabled and available for a queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShuffleState {
    /// Shuffle is enabled and order is randomized.
    Enabled,
    /// Shuffle is disabled but could be enabled.
    Disabled,
    /// Shuffle is not available (e.g. empty queue, or queue not shufflable).
    Unavailable,
}

/// Playback capability + state.
/// - `Linear`: not shufflable.
/// - `Shuffle`: shufflable; when enabled, maintains a permutation + cursor.
#[derive(Debug)]
pub enum PlaybackMode {
    /// Fixed linear order.
    Linear,
    /// Shuffled order with cursor tracking the playback position.
    Shuffle {
        /// Current shuffle state.
        state: ShuffleState,
        /// Playback permutation (track indices).
        order: Vec<usize>,
        /// Inverse mapping: inv[track_idx] = position in `order`.
        inv: Vec<usize>,
        /// Cursor in playback order (index into `order`).
        cursor: usize,
    },
}

/// Queue of tracks with optional shuffle ordering.
#[derive(Debug)]
pub struct TrackQueue {
    /// Stored tracks in insertion order.
    tracks: Vec<AppTrack>,
    /// Current track index (index into `tracks`).
    current_idx: usize,
    /// Playback ordering and state.
    mode: PlaybackMode,
}

/// Iterator over playback order indices.
pub enum OrderIter<'a> {
    /// Linear indices.
    Linear(std::ops::Range<usize>),
    /// Shuffled indices.
    Shuffled(std::iter::Copied<std::slice::Iter<'a, usize>>),
}

impl<'a> Iterator for OrderIter<'a> {
    type Item = usize;

    /// Return the next playback index.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OrderIter::Linear(it) => it.next(),
            OrderIter::Shuffled(it) => it.next(),
        }
    }
}

impl TrackQueue {
    /// Create an empty linear (non-shufflable) queue.
    pub fn new_linear() -> Self {
        Self {
            tracks: Vec::new(),
            current_idx: 0,
            mode: PlaybackMode::Linear,
        }
    }

    /// Create an empty shufflable queue (shuffle unavailable until tracks exist).
    pub fn new_shufflable() -> Self {
        Self {
            tracks: Vec::new(),
            current_idx: 0,
            mode: PlaybackMode::Shuffle {
                state: ShuffleState::Unavailable,
                order: Vec::new(),
                inv: Vec::new(),
                cursor: 0,
            },
        }
    }

    /// Return the number of tracks in the queue.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Return true if the queue has no tracks.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Return all tracks in insertion order.
    pub fn tracks(&self) -> &[AppTrack] {
        &self.tracks
    }

    /// Return the current track index (in insertion order).
    pub fn current_track_index(&self) -> Option<usize> {
        (self.current_idx < self.tracks.len()).then_some(self.current_idx)
    }

    /// Return the current track.
    pub fn current_track(&self) -> Option<&AppTrack> {
        self.tracks.get(self.current_idx)
    }

    /// Current index in *playback order* (linear index or shuffle cursor).
    pub fn current_playback_index(&self) -> Option<usize> {
        if self.tracks.is_empty() || self.current_idx >= self.tracks.len() {
            return None;
        }

        match &self.mode {
            PlaybackMode::Linear => Some(self.current_idx),
            PlaybackMode::Shuffle { state, cursor, .. } => match state {
                ShuffleState::Enabled => Some(*cursor),
                ShuffleState::Disabled | ShuffleState::Unavailable => Some(self.current_idx),
            },
        }
    }

    /// Iterate indices in playback order.
    pub fn playback_order_indices(&self) -> OrderIter<'_> {
        match &self.mode {
            PlaybackMode::Linear => OrderIter::Linear(0..self.tracks.len()),
            PlaybackMode::Shuffle { state, order, .. } => {
                if *state == ShuffleState::Enabled && !order.is_empty() {
                    OrderIter::Shuffled(order.iter().copied())
                } else {
                    OrderIter::Linear(0..self.tracks.len())
                }
            }
        }
    }

    /// Select current track by *track index*.
    pub fn set_current_track_index(&mut self, idx: usize) {
        if idx >= self.tracks.len() {
            return;
        }
        self.current_idx = idx;

        if let PlaybackMode::Shuffle {
            state: ShuffleState::Enabled,
            inv,
            cursor,
            ..
        } = &mut self.mode
        {
            // inv is guaranteed valid by invariants
            *cursor = inv[self.current_idx];
        }

        self.debug_assert_invariants();
    }

    /// Select current track by *playback index* (linear index or shuffle cursor).
    pub fn set_current_playback_index(&mut self, playback_idx: usize) {
        if self.tracks.is_empty() {
            return;
        }

        match &mut self.mode {
            PlaybackMode::Linear => {
                if playback_idx < self.tracks.len() {
                    self.current_idx = playback_idx;
                }
            }
            PlaybackMode::Shuffle {
                state,
                order,
                cursor,
                ..
            } => {
                if *state == ShuffleState::Enabled && playback_idx < order.len() {
                    *cursor = playback_idx;
                    self.current_idx = order[*cursor];
                } else if playback_idx < self.tracks.len() {
                    // When shuffle isn't active, treat as linear selection.
                    self.current_idx = playback_idx;
                }
            }
        }

        self.debug_assert_invariants();
    }

    /// Remove all tracks and reset state.
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_idx = 0;
        self.sync_mode_after_tracks_changed();
        self.debug_assert_invariants();
    }

    /// Replace all tracks and reset state.
    pub fn set_tracks(&mut self, tracks: Vec<AppTrack>) {
        self.tracks = tracks;
        self.current_idx = 0;
        self.sync_mode_after_tracks_changed();
        self.debug_assert_invariants();
    }

    /// Append tracks from any iterator.
    pub fn extend_back<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AppTrack>,
    {
        let start = self.tracks.len();
        self.tracks.extend(iter);

        if self.tracks.len() == start {
            return;
        }

        match &mut self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => {
                if self.tracks.is_empty() {
                    // practically unreachable (we just added), but keep invariant logic complete
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                    *state = ShuffleState::Unavailable;
                    return;
                }

                // If it was unavailable, it becomes available-but-off by default.
                if *state == ShuffleState::Unavailable {
                    *state = ShuffleState::Disabled;
                }

                if *state == ShuffleState::Enabled {
                    // Keep existing shuffle stable: append new indices at the end.
                    order.extend(start..self.tracks.len());
                    Self::rebuild_inverse_from_order(inv, order);

                    // Ensure cursor still points to current track.
                    if self.current_idx < inv.len() {
                        *cursor = inv[self.current_idx];
                    }
                } else {
                    // Not enabled: do not maintain permutation.
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                }
            }
        }

        self.debug_assert_invariants();
    }

    /// Append a single track at the end.
    pub fn push_back(&mut self, track: AppTrack) {
        self.extend_back(std::iter::once(track));
    }

    /// Insert a single track at the given index.
    /// If more tracks must be inserted, prefer insert_many_at().
    pub fn insert_one_at(&mut self, idx: usize, track: AppTrack) {
        let insert_idx = idx.min(self.tracks.len());
        let len_before = self.tracks.len();

        self.tracks.insert(insert_idx, track);

        // adjust current track index
        if self.current_idx < len_before && self.current_idx >= insert_idx {
            self.current_idx += 1;
        }

        match &mut self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => {
                if self.tracks.is_empty() {
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                    *state = ShuffleState::Unavailable;
                } else if *state == ShuffleState::Unavailable {
                    *state = ShuffleState::Disabled;
                }

                if *state == ShuffleState::Enabled {
                    // Shift indices >= insert_idx
                    for e in order.iter_mut() {
                        if *e >= insert_idx {
                            *e += 1;
                        }
                    }
                    // Keep existing permutation stable; append the inserted track at end.
                    order.push(insert_idx);

                    Self::rebuild_inverse_from_order(inv, order);
                    *cursor = inv[self.current_idx];
                } else {
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                }
            }
        }

        self.debug_assert_invariants();
    }

    /// Insert many tracks at the given position.
    pub fn insert_many_at<I>(&mut self, idx: usize, tracks: I)
    where
        I: IntoIterator<Item = AppTrack>,
    {
        let mut incoming: Vec<AppTrack> = tracks.into_iter().collect();
        if incoming.is_empty() {
            return;
        }

        let len_before = self.tracks.len();
        let insert_idx = idx.min(len_before);
        let count = incoming.len();

        self.tracks
            .splice(insert_idx..insert_idx, incoming.drain(..));

        // adjust current track index
        if len_before == 0 {
            self.current_idx = 0;
        } else if self.current_idx >= insert_idx {
            self.current_idx += count;
        }

        match &mut self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => {
                if self.tracks.is_empty() {
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                    *state = ShuffleState::Unavailable;
                } else if *state == ShuffleState::Unavailable {
                    *state = ShuffleState::Disabled;
                }

                if *state == ShuffleState::Enabled {
                    // Shift indices >= insert_idx by count.
                    for e in order.iter_mut() {
                        if *e >= insert_idx {
                            *e += count;
                        }
                    }
                    // Append inserted indices in order to keep shuffle stable.
                    order.extend(insert_idx..(insert_idx + count));

                    Self::rebuild_inverse_from_order(inv, order);
                    *cursor = inv[self.current_idx];
                } else {
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                }
            }
        }

        self.debug_assert_invariants();
    }

    /// Insert a single track after the current track.
    /// If more then one track must be inserted prefer insert_many_after_current()
    pub fn insert_one_after_current(&mut self, track: AppTrack) {
        self.insert_many_after_current(std::iter::once(track));
    }

    /// Insert many tacks after the current track.
    pub fn insert_many_after_current<I>(&mut self, tracks: I)
    where
        I: IntoIterator<Item = AppTrack>,
    {
        let len = self.tracks.len();
        let insert_idx = if len == 0 {
            0
        } else {
            let cur = self.current_idx.min(len.saturating_sub(1));
            (cur + 1).min(len)
        };
        self.insert_many_at(insert_idx, tracks);
    }

    /// Remove the track at the given index and return it.
    pub fn remove_at(&mut self, idx: usize) -> Option<AppTrack> {
        if idx >= self.tracks.len() {
            return None;
        }

        let removed = self.tracks.remove(idx);

        // adjust current track index
        if self.current_idx > idx {
            self.current_idx -= 1;
        } else if self.current_idx >= self.tracks.len() {
            self.current_idx = self.tracks.len().saturating_sub(1);
        }

        match &mut self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => {
                if self.tracks.is_empty() {
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                    *state = ShuffleState::Unavailable;
                } else if *state == ShuffleState::Unavailable {
                    *state = ShuffleState::Disabled;
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                } else if *state == ShuffleState::Enabled {
                    // Remove idx from order (O(1) locate via inv)
                    let pos_in_order = inv[idx];
                    if pos_in_order < order.len() && order[pos_in_order] == idx {
                        order.remove(pos_in_order);
                    } else {
                        // fallback if desynced
                        if let Some(p) = order.iter().position(|&t| t == idx) {
                            order.remove(p);
                        }
                    }

                    // Shift indices > idx down by 1
                    for t in order.iter_mut() {
                        if *t > idx {
                            *t -= 1;
                        }
                    }

                    Self::rebuild_inverse_from_order(inv, order);

                    // Re-anchor cursor to current track.
                    *cursor = inv[self.current_idx];
                } else {
                    // Disabled: do not maintain permutation
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                }
            }
        }

        self.debug_assert_invariants();
        Some(removed)
    }

    /// Return the current shuffle state.
    pub fn shuffle_state(&self) -> ShuffleState {
        match &self.mode {
            PlaybackMode::Linear => ShuffleState::Unavailable,
            PlaybackMode::Shuffle { state, .. } => *state,
        }
    }

    /// Return true if shuffle is enabled.
    pub fn is_shuffle_enabled(&self) -> bool {
        matches!(
            &self.mode,
            PlaybackMode::Shuffle {
                state: ShuffleState::Enabled,
                ..
            }
        )
    }

    /// Enable shuffle; keeps the current track at the front of the playback order.
    pub fn enable_shuffle(&mut self) {
        let len = self.tracks.len();
        match &mut self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle { state, .. } => {
                if len == 0 {
                    *state = ShuffleState::Unavailable;
                    self.sync_mode_after_tracks_changed();
                    self.debug_assert_invariants();
                    return;
                }
                *state = ShuffleState::Enabled;
                self.rebuild_shuffle_keep_current_front();
                self.debug_assert_invariants();
            }
        }
    }

    /// Disable shuffle and revert to linear playback order.
    pub fn disable_shuffle(&mut self) {
        if let PlaybackMode::Shuffle {
            state,
            order,
            inv,
            cursor,
        } = &mut self.mode
        {
            if self.tracks.is_empty() {
                *state = ShuffleState::Unavailable;
            } else {
                *state = ShuffleState::Disabled;
            }
            order.clear();
            inv.clear();
            *cursor = 0;
        }

        self.debug_assert_invariants();
    }

    /// Advance to the next track, optionally wrapping to the start.
    pub fn next(&mut self, wrap: bool) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        match &mut self.mode {
            PlaybackMode::Linear => Self::advance_linear_idx(&mut self.current_idx, len, wrap),
            PlaybackMode::Shuffle {
                state,
                order,
                cursor,
                ..
            } => {
                if *state == ShuffleState::Enabled && !order.is_empty() {
                    if *cursor + 1 < order.len() {
                        *cursor += 1;
                        self.current_idx = order[*cursor];
                        Some(self.current_idx)
                    } else if wrap {
                        *cursor = 0;
                        self.current_idx = order[*cursor];
                        Some(self.current_idx)
                    } else {
                        None
                    }
                } else {
                    // behave linearly when shuffle is disabled/unavailable
                    Self::advance_linear_idx(&mut self.current_idx, len, wrap)
                }
            }
        }
    }

    /// Move to the previous track, optionally wrapping to the end.
    pub fn prev(&mut self, wrap: bool) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        match &mut self.mode {
            PlaybackMode::Linear => Self::retreat_linear_idx(&mut self.current_idx, len, wrap),
            PlaybackMode::Shuffle {
                state,
                order,
                cursor,
                ..
            } => {
                if *state == ShuffleState::Enabled && !order.is_empty() {
                    if *cursor > 0 {
                        *cursor -= 1;
                        self.current_idx = order[*cursor];
                        Some(self.current_idx)
                    } else if wrap {
                        *cursor = order.len() - 1;
                        self.current_idx = order[*cursor];
                        Some(self.current_idx)
                    } else {
                        None
                    }
                } else {
                    // behave linearly when shuffle is disabled/unavailable
                    Self::retreat_linear_idx(&mut self.current_idx, len, wrap)
                }
            }
        }
    }

    /// Return the track at the given index.
    pub fn track_at(&self, idx: usize) -> Option<&AppTrack> {
        self.tracks.get(idx)
    }

    /// Ensure ordering state is valid after track changes.
    fn sync_mode_after_tracks_changed(&mut self) {
        match &mut self.mode {
            PlaybackMode::Linear => {
                // Keep cursor valid in linear mode.
                if self.tracks.is_empty() {
                    self.current_idx = 0;
                } else if self.current_idx >= self.tracks.len() {
                    self.current_idx = self.tracks.len() - 1;
                }
            }
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => {
                if self.tracks.is_empty() {
                    *state = ShuffleState::Unavailable;
                    order.clear();
                    inv.clear();
                    *cursor = 0;
                    self.current_idx = 0;
                    return;
                }

                if self.current_idx >= self.tracks.len() {
                    self.current_idx = self.tracks.len() - 1;
                }

                if *state == ShuffleState::Unavailable {
                    *state = ShuffleState::Disabled;
                }

                match *state {
                    ShuffleState::Enabled => {
                        self.rebuild_shuffle_keep_current_front();
                    }
                    ShuffleState::Disabled => {
                        order.clear();
                        inv.clear();
                        *cursor = 0;
                    }
                    ShuffleState::Unavailable => {
                        // handled above
                    }
                }
            }
        }
    }

    /// Advance a linear index, optionally wrapping.
    fn advance_linear_idx(current_idx: &mut usize, len: usize, wrap: bool) -> Option<usize> {
        if len == 0 {
            return None;
        }
        if *current_idx + 1 < len {
            *current_idx += 1;
            Some(*current_idx)
        } else if wrap {
            *current_idx = 0;
            Some(*current_idx)
        } else {
            None
        }
    }

    /// Retreat a linear index, optionally wrapping.
    fn retreat_linear_idx(current_idx: &mut usize, len: usize, wrap: bool) -> Option<usize> {
        if len == 0 {
            return None;
        }
        if *current_idx > 0 {
            *current_idx -= 1;
            Some(*current_idx)
        } else if wrap {
            *current_idx = len - 1;
            Some(*current_idx)
        } else {
            None
        }
    }

    /// Rebuild a shuffled order while keeping the current track first.
    fn rebuild_shuffle_keep_current_front(&mut self) {
        let len = self.tracks.len();
        let cur = self.current_idx.min(len.saturating_sub(1));

        let PlaybackMode::Shuffle {
            state,
            order,
            inv,
            cursor,
        } = &mut self.mode
        else {
            return;
        };

        if len == 0 {
            *state = ShuffleState::Unavailable;
            order.clear();
            inv.clear();
            *cursor = 0;
            self.current_idx = 0;
            return;
        }

        let mut new_order: Vec<usize> = (0..len).collect();
        if len > 1 {
            new_order.shuffle(&mut rand::thread_rng());
        }

        // Keep current track at front for stable playback.
        if let Some(i) = new_order.iter().position(|&p| p == cur) {
            new_order.swap(0, i);
        }

        *order = new_order;
        Self::rebuild_inverse_from_order(inv, order);

        *cursor = 0;
        self.current_idx = order[*cursor];
        *state = ShuffleState::Enabled;
    }

    /// Rebuild inverse mapping from an order array.
    fn rebuild_inverse_from_order(inv: &mut Vec<usize>, order: &[usize]) {
        inv.clear();
        inv.resize(order.len(), 0);
        for (i, &track_idx) in order.iter().enumerate() {
            inv[track_idx] = i;
        }
    }

    /// Debug-only invariant checks for queue state.
    fn debug_assert_invariants(&self) {
        debug_assert!(
            self.tracks.is_empty() || self.current_idx < self.tracks.len(),
            "current_idx out of bounds"
        );

        match &self.mode {
            PlaybackMode::Linear => {}
            PlaybackMode::Shuffle {
                state,
                order,
                inv,
                cursor,
            } => match *state {
                ShuffleState::Unavailable => {
                    debug_assert!(self.tracks.is_empty(), "unavailable but tracks non-empty");
                    debug_assert!(order.is_empty(), "unavailable but order not empty");
                    debug_assert!(inv.is_empty(), "unavailable but inv not empty");
                    debug_assert!(*cursor == 0, "unavailable but cursor != 0");
                }
                ShuffleState::Disabled => {
                    debug_assert!(!self.tracks.is_empty(), "disabled but tracks empty");
                    debug_assert!(order.is_empty(), "disabled but order not empty");
                    debug_assert!(inv.is_empty(), "disabled but inv not empty");
                    debug_assert!(*cursor == 0, "disabled but cursor != 0");
                }
                ShuffleState::Enabled => {
                    debug_assert!(
                        order.len() == self.tracks.len(),
                        "enabled but order length != tracks length"
                    );
                    debug_assert!(
                        inv.len() == self.tracks.len(),
                        "enabled but inv length != tracks length"
                    );
                    debug_assert!(*cursor < order.len(), "cursor out of bounds");
                    debug_assert!(
                        order[*cursor] == self.current_idx,
                        "cursor/current_idx mismatch"
                    );
                    // sanity check inverse mapping
                    debug_assert!(inv[self.current_idx] == *cursor, "inv mapping mismatch");
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// Create a lightweight test track.
    fn track(id: u32) -> AppTrack {
        AppTrack {
            id,
            title: format!("t{id}"),
            artist: "artist".to_string(),
            album_title: "album".to_string(),
            path_buf: PathBuf::from(format!("/tmp/{id}.mp3")),
            cover_art: None,
        }
    }

    /// Assert that `order` is a permutation of 0..len.
    fn assert_is_permutation(order: &[usize], len: usize) {
        assert_eq!(order.len(), len);
        let mut set = HashSet::new();
        for &i in order {
            assert!(i < len);
            set.insert(i);
        }
        assert_eq!(set.len(), len);
    }

    /// Linear navigation supports forward/back and wrapping.
    #[test]
    fn linear_next_prev_and_wrap() {
        let mut q = TrackQueue::new_linear();
        q.set_tracks(vec![track(1), track(2), track(3)]);

        assert_eq!(q.current_track_index(), Some(0));
        assert_eq!(q.next(false), Some(1));
        assert_eq!(q.current_track_index(), Some(1));
        assert_eq!(q.prev(false), Some(0));
        assert_eq!(q.current_track_index(), Some(0));
        assert_eq!(q.prev(false), None);

        q.set_current_track_index(2);
        assert_eq!(q.next(false), None);
        assert_eq!(q.next(true), Some(0));
        assert_eq!(q.current_track_index(), Some(0));
    }

    /// Enabling shuffle keeps current track at the front of playback order.
    #[test]
    fn shuffle_keeps_current_front() {
        let mut q = TrackQueue::new_shufflable();
        q.set_tracks(vec![track(1), track(2), track(3), track(4)]);
        q.set_current_track_index(2);

        q.enable_shuffle();
        assert_eq!(q.shuffle_state(), ShuffleState::Enabled);

        let order: Vec<usize> = q.playback_order_indices().collect();
        assert_eq!(order.first().copied(), Some(2));
        assert_is_permutation(&order, 4);

        q.disable_shuffle();
        let linear: Vec<usize> = q.playback_order_indices().collect();
        assert_eq!(linear, vec![0, 1, 2, 3]);
    }

    /// Removing and inserting in shuffle keeps valid order indices.
    #[test]
    fn shuffle_remove_and_insert_keeps_consistent_order() {
        let mut q = TrackQueue::new_shufflable();
        q.set_tracks(vec![track(1), track(2), track(3), track(4)]);
        q.enable_shuffle();

        let _ = q.remove_at(1);
        let order: Vec<usize> = q.playback_order_indices().collect();
        assert_is_permutation(&order, 3);
        assert!(q.current_track_index().unwrap_or(0) < q.len());

        q.insert_one_at(1, track(5));
        let order_after: Vec<usize> = q.playback_order_indices().collect();
        assert_is_permutation(&order_after, 4);
    }
}
