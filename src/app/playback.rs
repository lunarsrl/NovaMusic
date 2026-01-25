// SPDX-License-Identifier: GPL-2.0-or-later
//! Playback manager coordinating up-next and context queues.

use crate::app::queue::{ShuffleState, TrackQueue};
use crate::app::AppTrack;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LoopState {
    /// Loop every track on a Natural track finish.
    LoopingTrack,
    /// Loop the context queue.
    LoopingQueue,
    /// Don't loop.
    NotLooping,
    /// When the playback manager has no tracks.
    Unavailable,
}

/// Active queue used for the current track selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveQueue {
    /// Tracks queued to play next.
    UpNext,
    /// Context tracks (album/playlist) that resume after up-next.
    Context,
}

/// Orchestrates playback across the up-next and context queues.
pub struct PlaybackManager {
    /// Context queue (shufflable).
    context: TrackQueue,
    /// Up-next queue (linear).
    up_next: TrackQueue,
    /// Current active queue for playback.
    active: ActiveQueue,
    /// Whether the up-next queue has started playing.
    up_next_started: bool,
    /// Whether the context queue has finished playback.
    context_finished: bool,
    /// Current looping state.
    loop_state: LoopState,
}

/// Reason for advancing playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvanceReason {
    /// Track ended naturally.
    Natural,
    /// User pressed skip/next.
    Skip,
    /// User pressed previous.
    Previous,
}

/// Result of attempting to advance playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvanceOutcome {
    /// Restart the current track (LoopingTrack).
    RestartCurrent,
    /// Moved to a different track.
    Moved,
    /// No change.
    NoChange,
    /// Reached end in NotLooping; reset to start and pause.
    Ended,
}

impl PlaybackManager {
    /// Two-queue playback:
    /// - up_next plays next; context resumes after up_next
    /// - context is shufflable, indices are interpreted in playback order
    pub fn new() -> Self {
        let mut pm = Self {
            context: TrackQueue::new_shufflable(),
            up_next: TrackQueue::new_linear(),
            active: ActiveQueue::Context,
            up_next_started: false,
            context_finished: false,
            loop_state: LoopState::Unavailable,
        };
        pm.sync_active_queue();
        pm.sync_loop_state_availability();
        pm
    }

    /// Return true if both queues are empty.
    pub fn is_empty(&self) -> bool {
        self.up_next.is_empty() && self.context.is_empty()
    }

    /// Return total number of tracks across both queues.
    pub fn total_len(&self) -> usize {
        self.up_next.len() + self.context.len()
    }

    /// Return the number of tracks in the up-next queue.
    pub fn up_next_len(&self) -> usize {
        self.up_next.len()
    }

    /// Return the number of tracks in the context queue.
    pub fn context_len(&self) -> usize {
        self.context.len()
    }

    /// Return the currently selected track.
    pub fn current_track(&self) -> Option<&AppTrack> {
        match self.active {
            ActiveQueue::UpNext => self.up_next.current_track(),
            ActiveQueue::Context => self.context.current_track(),
        }
    }

    /// Current index in the unified list:
    /// up_next: [0..up_next_len)
    /// context: [up_next_len..up_next_len+context_len) interpreted in *context playback order*
    pub fn current_global_index(&self) -> Option<usize> {
        match self.active {
            ActiveQueue::UpNext => self.up_next.current_playback_index(),
            ActiveQueue::Context => self
                .context
                .current_playback_index()
                .map(|i| self.up_next.len() + i),
        }
    }

    /// Current looping state.
    pub fn loop_state(&self) -> LoopState {
        self.loop_state
    }

    /// Toggle current looping state.
    ///
    /// * NotLooping -> LoopingTrack
    /// * LoopingTrack -> LoopingQueue
    /// * LoopingQueue -> NotLooping
    pub fn toggle_loop_state(&mut self) -> LoopState {
        self.sync_loop_state_availability();
        if self.loop_state == LoopState::Unavailable {
            return self.loop_state;
        }

        self.loop_state = match self.loop_state {
            LoopState::NotLooping => LoopState::LoopingTrack,
            LoopState::LoopingTrack => LoopState::LoopingQueue,
            LoopState::LoopingQueue => LoopState::NotLooping,
            LoopState::Unavailable => LoopState::Unavailable,
        };
        self.loop_state
    }

    /// Clear the up-next queue.
    pub fn clear_up_next(&mut self) {
        self.up_next.clear();
        self.up_next_started = false;
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Clear both queues.
    pub fn clear_all(&mut self) {
        self.up_next.clear();
        self.context.clear();
        self.up_next_started = false;
        self.context_finished = false;
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Replace context contents (playlist/album) and clear up_next.
    pub fn set_context_tracks(&mut self, tracks: Vec<AppTrack>) {
        self.context.set_tracks(tracks);
        self.up_next.clear();
        self.up_next_started = false;
        self.context_finished = false;
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Queue to play immediately after the current track (front of up_next).
    /// If currently in context, this becomes the next track overall.
    pub fn queue_next(&mut self, track: AppTrack) {
        let next_idx = self.up_next_index_after_current();

        self.up_next.insert_one_at(next_idx, track);
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Queue multiple tracks to play immediately after the current track.
    pub fn queue_next_many<I>(&mut self, tracks: I)
    where
        I: IntoIterator<Item = AppTrack>,
    {
        let insert_at = self.up_next_index_after_current();
        self.up_next.insert_many_at(insert_at, tracks);
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Queue at the end of up_next.
    pub fn queue_last(&mut self, track: AppTrack) {
        self.up_next.push_back(track);
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Queue multiple tracks at the end of up-next.
    pub fn queue_last_many<I>(&mut self, tracks: I)
    where
        I: IntoIterator<Item = AppTrack>,
    {
        self.up_next.extend_back(tracks);
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Remove by unified index (context indices are interpreted in context playback order).
    pub fn remove_by_global_index(&mut self, global_index: usize) {
        let up_len = self.up_next.len();

        if global_index < up_len {
            let _ = self.up_next.remove_at(global_index);
            if self.up_next.is_empty() {
                self.up_next_started = false;
            }
            self.sync_active_queue();
            self.sync_loop_state_availability();
            return;
        }

        let ctx_playback_index = global_index - up_len;
        self.remove_context_by_playback_index(ctx_playback_index);
        if self.context.is_empty() {
            self.context_finished = false;
        }
        self.sync_active_queue();
        self.sync_loop_state_availability();
    }

    /// Advance playback according to the reason and loop mode.
    pub fn advance(&mut self, reason: AdvanceReason) -> AdvanceOutcome {
        let loop_state = match self.loop_state {
            LoopState::Unavailable => LoopState::NotLooping,
            other => other,
        };

        let effective_loop = match reason {
            AdvanceReason::Natural => loop_state,
            AdvanceReason::Skip | AdvanceReason::Previous => match loop_state {
                LoopState::LoopingQueue => LoopState::LoopingQueue,
                _ => LoopState::NotLooping,
            },
        };

        if matches!(effective_loop, LoopState::LoopingTrack)
            && matches!(reason, AdvanceReason::Natural)
        {
            return AdvanceOutcome::RestartCurrent;
        }

        let wrap_context = matches!(effective_loop, LoopState::LoopingQueue);
        let moved = match reason {
            AdvanceReason::Natural | AdvanceReason::Skip => self.advance_next(wrap_context),
            AdvanceReason::Previous => self.advance_prev(wrap_context),
        };

        if moved {
            return AdvanceOutcome::Moved;
        }

        if matches!(reason, AdvanceReason::Natural) {
            self.reset_to_start();
            return AdvanceOutcome::Ended;
        }

        AdvanceOutcome::NoChange
    }

    /// Internal helper for advance(): Advance to the next track.
    /// Returns true if the current track changed, false if already at end.
    fn advance_next(&mut self, wrap_context: bool) -> bool {
        self.sync_active_queue();

        match self.active {
            ActiveQueue::UpNext => {
                self.up_next_started = true;
                if self.up_next.next(false).is_some() {
                    return true;
                }

                // up_next ended -> switch to context
                self.active = ActiveQueue::Context;

                if self.context.is_empty() {
                    self.sync_active_queue();
                    return false;
                }

                if self.context_finished {
                    if wrap_context {
                        self.context.set_current_playback_index(0);
                        self.context_finished = false;
                        return true;
                    }
                    self.sync_active_queue();
                    return false;
                }

                true
            }
            ActiveQueue::Context => {
                let advanced = self.context.next(false).is_some();
                if advanced {
                    self.context_finished = false;
                    if self.switch_to_up_next_after_context() {
                        return true;
                    }
                    return true;
                }

                // context ended
                self.context_finished = true;

                if self.switch_to_up_next_after_context() {
                    return true;
                }

                if wrap_context && !self.context.is_empty() {
                    self.context.set_current_playback_index(0);
                    self.context_finished = false;
                    return true;
                }

                false
            }
        }
    }

    /// Internal helper for advance(): Go to previous track.
    /// Returns true if the current track changed, false if already at start.
    fn advance_prev(&mut self, wrap_context: bool) -> bool {
        self.sync_active_queue();

        match self.active {
            ActiveQueue::UpNext => {
                self.up_next_started = true;
                if self.up_next.prev(false).is_some() {
                    return true;
                }
                if wrap_context && !self.context.is_empty() {
                    self.active = ActiveQueue::Context;
                    self.context
                        .set_current_playback_index(self.context.len().saturating_sub(1));
                    self.context_finished = false;
                    return true;
                }
                false
            }
            ActiveQueue::Context => {
                if self.context.is_empty() {
                    return false;
                }

                self.context_finished = false;
                if self.context.prev(false).is_some() {
                    return true;
                }

                if wrap_context {
                    self.context
                        .set_current_playback_index(self.context.len().saturating_sub(1));
                    return true;
                }

                if self.up_next_started && !self.up_next.is_empty() {
                    self.active = ActiveQueue::UpNext;
                    if let Some(cur) = self.up_next.current_track_index() {
                        self.up_next.set_current_track_index(cur);
                    }
                    return true;
                }

                false
            }
        }
    }

    /// Set current track by unified index (context indices are interpreted in context playback order).
    pub fn set_current_global_index(&mut self, global_index: usize) {
        let up_len = self.up_next.len();

        if global_index < up_len {
            self.active = ActiveQueue::UpNext;
            self.up_next.set_current_track_index(global_index);
            self.up_next_started = true;
            return;
        }

        let ctx_i = global_index - up_len;
        if ctx_i < self.context.len() {
            self.active = ActiveQueue::Context;
            self.context.set_current_playback_index(ctx_i);
            self.context_finished = false;
        }

        self.sync_active_queue();
    }

    /// Return the context queue shuffle state.
    pub fn shuffle_state(&self) -> ShuffleState {
        self.context.shuffle_state()
    }

    /// Return true if context shuffle is enabled.
    pub fn is_shuffle_enabled(&self) -> bool {
        self.context.is_shuffle_enabled()
    }

    /// Toggle context shuffle.
    /// If shuffle is unavailable, does nothing.
    pub fn toggle_shuffle(&mut self) {
        if self.context.shuffle_state() == ShuffleState::Unavailable {
            return;
        }

        if self.context.is_shuffle_enabled() {
            self.context.disable_shuffle();
        } else {
            self.context.enable_shuffle();
        }
    }

    /// Global indices of tracks already played (excluding current).
    pub fn played_global_indices(&self) -> Vec<usize> {
        let (up_played, ctx_played, _, _) = self.split_indices();
        up_played
            .into_iter()
            .chain(ctx_played.into_iter())
            .collect()
    }

    /// Global indices of tracks still upcoming (excluding current).
    pub fn upcoming_global_indices(&self) -> Vec<usize> {
        let (_, _, up_upcoming, ctx_upcoming) = self.split_indices();
        up_upcoming
            .into_iter()
            .chain(ctx_upcoming.into_iter())
            .collect()
    }

    /// Per-queue global indices of already played tracks (excluding current).
    pub fn played_up_next_global_indices(&self) -> Vec<usize> {
        let (up_played, _, _, _) = self.split_indices();
        up_played
    }

    pub fn played_context_global_indices(&self) -> Vec<usize> {
        let (_, ctx_played, _, _) = self.split_indices();
        ctx_played
    }

    /// Per-queue global indices of upcoming tracks (excluding current).
    pub fn upcoming_up_next_global_indices(&self) -> Vec<usize> {
        let (_, _, up_upcoming, _) = self.split_indices();
        up_upcoming
    }

    pub fn upcoming_context_global_indices(&self) -> Vec<usize> {
        let (_, _, _, ctx_upcoming) = self.split_indices();
        ctx_upcoming
    }

    /// Lookup a track by its unified global index.
    pub fn track_by_global_index(&self, global_index: usize) -> Option<&AppTrack> {
        let up_len = self.up_next.len();
        if global_index < up_len {
            return self.up_next.track_at(global_index);
        }

        let playback_index = global_index - up_len;
        self.context_track_by_playback_index(playback_index)
    }

    /// Tracks starting from the current track (includes current), across both queues.
    pub fn tracks_from_current(&self) -> Vec<AppTrack>
    where
        AppTrack: Clone,
    {
        let mut out = Vec::new();

        match self.active {
            ActiveQueue::UpNext => {
                let up_len = self.up_next.len();
                if let Some(cur) = self.up_next.current_track_index() {
                    for i in cur..up_len {
                        if let Some(t) = self.up_next.track_at(i) {
                            out.push(t.clone());
                        }
                    }
                }
                if !self.context_finished {
                    let ctx_start = self.context.current_playback_index().unwrap_or(0);
                    self.extend_context_from_playback_index(ctx_start, &mut out);
                }
            }
            ActiveQueue::Context => {
                if self.context.is_empty() {
                    return out;
                }
                if self.context_finished {
                    if let Some(start) = self.up_next_next_index() {
                        self.extend_up_next_from_index(start, &mut out);
                    }
                    return out;
                }
                let cur = self.context.current_playback_index().unwrap_or(0);
                if let Some(track) = self.context_track_by_playback_index(cur) {
                    out.push(track.clone());
                }

                if let Some(start) = self.up_next_next_index() {
                    self.extend_up_next_from_index(start, &mut out);
                }

                self.extend_context_from_playback_index(cur + 1, &mut out);
            }
        }

        out
    }

    /// Context tracks starting from the current context track (includes current),
    /// interpreted in context playback order.
    pub fn context_tracks_from_current(&self) -> Vec<AppTrack>
    where
        AppTrack: Clone,
    {
        let mut out = Vec::new();

        if self.context.is_empty() || self.context_finished {
            return out;
        }

        let start = self.context.current_playback_index().unwrap_or(0);
        self.extend_context_from_playback_index(start, &mut out);
        out
    }

    /// Up-next tracks starting from the current up-next track (includes current),
    /// in linear order.
    pub fn up_next_tracks_from_current(&self) -> Vec<AppTrack>
    where
        AppTrack: Clone,
    {
        let mut out = Vec::new();

        let up_len = self.up_next.len();
        if up_len == 0 {
            return out;
        }

        let start = match self.active {
            ActiveQueue::UpNext => self.up_next.current_track_index().unwrap_or(0),
            ActiveQueue::Context => {
                if self.up_next_started {
                    let cur = self.up_next.current_track_index().unwrap_or(0);
                    cur.saturating_add(1)
                } else {
                    0
                }
            }
        };
        if start >= up_len {
            return out;
        }
        for i in start..up_len {
            if let Some(t) = self.up_next.track_at(i) {
                out.push(t.clone());
            }
        }

        out
    }

    /// Ensure the active queue is valid after mutations.
    fn sync_active_queue(&mut self) {
        if self.active == ActiveQueue::UpNext && self.up_next.is_empty() {
            self.active = ActiveQueue::Context;
        } else if self.active == ActiveQueue::Context
            && self.context.is_empty()
            && !self.up_next.is_empty()
        {
            self.active = ActiveQueue::UpNext;
            self.up_next_started = true;
        }
    }

    /// Keep looping availability in sync with queue contents.
    fn sync_loop_state_availability(&mut self) {
        if self.is_empty() {
            self.loop_state = LoopState::Unavailable;
        } else if self.loop_state == LoopState::Unavailable {
            self.loop_state = LoopState::NotLooping;
        }
    }

    /// Reset the current track to the start of the context queue (or up-next if context is empty).
    fn reset_to_start(&mut self) {
        if !self.context.is_empty() {
            self.active = ActiveQueue::Context;
            self.context.set_current_playback_index(0);
            self.context_finished = false;
            return;
        }

        if !self.up_next.is_empty() {
            self.active = ActiveQueue::UpNext;
            self.up_next.set_current_track_index(0);
            self.up_next_started = false;
        }
    }

    /// Return the context playback order as track indices.
    fn context_playback_order_vec(&self) -> Vec<usize> {
        self.context.playback_order_indices().collect()
    }

    /// Lookup a context track by its playback-order index.
    fn context_track_by_playback_index(&self, playback_index: usize) -> Option<&AppTrack> {
        let order = self.context_playback_order_vec();
        order
            .get(playback_index)
            .and_then(|&track_idx| self.context.track_at(track_idx))
    }

    /// Remove a context track by its playback-order index.
    fn remove_context_by_playback_index(&mut self, playback_index: usize) {
        if self.context.is_empty() {
            return;
        }
        let order = self.context_playback_order_vec();
        if playback_index >= order.len() {
            return;
        }
        let track_idx = order[playback_index];
        let _ = self.context.remove_at(track_idx);
    }

    /// Append context tracks from a playback-order index into `out`.
    fn extend_context_from_playback_index(
        &self,
        start_playback_index: usize,
        out: &mut Vec<AppTrack>,
    ) where
        AppTrack: Clone,
    {
        if self.context.is_empty() {
            return;
        }

        let order = self.context.playback_order_indices().collect::<Vec<_>>();
        for playback_i in start_playback_index..order.len() {
            if let Some(t) = self.context.track_at(order[playback_i]) {
                out.push(t.clone());
            }
        }
    }

    /// Append up-next tracks from a linear index into `out`.
    fn extend_up_next_from_index(&self, start: usize, out: &mut Vec<AppTrack>)
    where
        AppTrack: Clone,
    {
        let up_len = self.up_next.len();
        if start >= up_len {
            return;
        }
        for i in start..up_len {
            if let Some(t) = self.up_next.track_at(i) {
                out.push(t.clone());
            }
        }
    }

    /// Compute the next up-next index to play, if any.
    fn up_next_next_index(&self) -> Option<usize> {
        if self.up_next.is_empty() {
            return None;
        }

        let cur = self.up_next.current_track_index().unwrap_or(0);
        if self.active == ActiveQueue::UpNext || self.up_next_started {
            let next = cur + 1;
            (next < self.up_next.len()).then_some(next)
        } else {
            Some(cur)
        }
    }

    /// Switch playback to the next up-next track if available.
    fn switch_to_up_next_after_context(&mut self) -> bool {
        let Some(next_idx) = self.up_next_next_index() else {
            return false;
        };

        self.up_next.set_current_track_index(next_idx);
        self.active = ActiveQueue::UpNext;
        self.up_next_started = true;
        true
    }

    /// Determine the insertion position for "play next" items in up-next.
    fn up_next_index_after_current(&self) -> usize {
        match self.active {
            ActiveQueue::UpNext => {
                let cur = self.up_next.current_track_index().unwrap_or(0);
                (cur + 1).min(self.up_next.len())
            }
            ActiveQueue::Context => {
                if self.up_next_started {
                    let cur = self.up_next.current_track_index().unwrap_or(0);
                    (cur + 1).min(self.up_next.len())
                } else {
                    0
                }
            }
        }
    }

    /// Split unified indices into:
    /// (played_up_next, played_context, upcoming_up_next, upcoming_context)
    fn split_indices(&self) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>) {
        let up_len = self.up_next.len();
        let ctx_len = self.context.len();

        let mut played_up = Vec::new();
        let mut played_ctx = Vec::new();
        let mut upcoming_up = Vec::new();
        let mut upcoming_ctx = Vec::new();

        if up_len == 0 && ctx_len == 0 {
            return (played_up, played_ctx, upcoming_up, upcoming_ctx);
        }

        if up_len > 0 {
            let cur = self.up_next.current_track_index().unwrap_or(0);
            if self.active == ActiveQueue::UpNext {
                played_up.extend(0..cur);
                if cur + 1 < up_len {
                    upcoming_up.extend((cur + 1)..up_len);
                }
            } else if self.up_next_started {
                played_up.extend(0..=cur);
                if cur + 1 < up_len {
                    upcoming_up.extend((cur + 1)..up_len);
                }
            } else {
                upcoming_up.extend(0..up_len);
            }
        }

        if ctx_len > 0 {
            let base = up_len;
            if self.context_finished {
                played_ctx.extend(base..(base + ctx_len));
            } else {
                let cur = self.context.current_playback_index().unwrap_or(0);
                if self.active == ActiveQueue::Context {
                    played_ctx.extend(base..(base + cur));
                    if cur + 1 < ctx_len {
                        upcoming_ctx.extend((base + cur + 1)..(base + ctx_len));
                    }
                } else {
                    played_ctx.extend(base..(base + cur));
                    upcoming_ctx.extend((base + cur)..(base + ctx_len));
                }
            }
        }

        (played_up, played_ctx, upcoming_up, upcoming_ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Extract track ids for assertion convenience.
    fn ids(tracks: &[AppTrack]) -> Vec<u32> {
        tracks.iter().map(|t| t.id).collect()
    }

    /// Play-next items are consumed before resuming context.
    #[test]
    fn play_next_inserts_before_context_resume() {
        let mut pm = PlaybackManager::new();
        pm.set_context_tracks(vec![track(1), track(2), track(3)]);
        pm.queue_next_many(vec![track(10), track(11)]);

        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
        assert_eq!(ids(&pm.tracks_from_current()), vec![1, 10, 11, 2, 3]);

        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(10));
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(11));
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(2));
    }

    /// Upcoming lists exclude the current track and reflect queue priorities.
    #[test]
    fn upcoming_lists_exclude_current_and_prioritize_up_next() {
        let mut pm = PlaybackManager::new();
        pm.set_context_tracks(vec![track(1), track(2), track(3)]);
        pm.queue_last_many(vec![track(10), track(11)]);

        let up_next = pm.upcoming_up_next_global_indices();
        let context = pm.upcoming_context_global_indices();

        assert_eq!(up_next, vec![0, 1]);
        assert_eq!(context, vec![3, 4]);
    }

    /// Selecting a history item resumes playback from that position.
    #[test]
    fn history_jump_resumes_from_selected_track() {
        let mut pm = PlaybackManager::new();
        pm.set_context_tracks(vec![track(1), track(2), track(3)]);

        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(2));

        let played = pm.played_global_indices();
        assert_eq!(played, vec![0]);

        pm.set_current_global_index(0);
        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
        assert_eq!(ids(&pm.tracks_from_current()), vec![1, 2, 3]);
    }

    /// Up-next plays before context on advance.
    #[test]
    fn up_next_plays_before_context_wraps_back() {
        let mut pm = PlaybackManager::new();
        pm.set_context_tracks(vec![track(1), track(2)]);
        pm.queue_next(track(10));

        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(10));
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(2));
    }

    /// LoopingTrack restarts the current track on natural end.
    #[test]
    fn looping_track_restarts_on_natural_end() {
        let mut pm = PlaybackManager::new();
        pm.loop_state = LoopState::LoopingTrack;
        pm.set_context_tracks(vec![track(1), track(2)]);

        let outcome = pm.advance(AdvanceReason::Natural);
        assert_eq!(outcome, AdvanceOutcome::RestartCurrent);
        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
    }

    /// Skip does not wrap when not looping the queue.
    #[test]
    fn skip_does_not_wrap_without_loop_queue() {
        let mut pm = PlaybackManager::new();
        pm.loop_state = LoopState::NotLooping;
        pm.set_context_tracks(vec![track(1)]);

        let outcome = pm.advance(AdvanceReason::Skip);
        assert_eq!(outcome, AdvanceOutcome::NoChange);
        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
    }

    /// Natural end in NotLooping resets to start and pauses.
    #[test]
    fn natural_end_resets_to_start_when_not_looping() {
        let mut pm = PlaybackManager::new();
        pm.loop_state = LoopState::NotLooping;
        pm.set_context_tracks(vec![track(1), track(2)]);
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(2));

        let outcome = pm.advance(AdvanceReason::Natural);
        assert_eq!(outcome, AdvanceOutcome::Ended);
        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
    }

    /// LoopingQueue wraps context but does not wrap up-next.
    #[test]
    fn looping_queue_wraps_context_only() {
        let mut pm = PlaybackManager::new();
        pm.loop_state = LoopState::LoopingQueue;
        pm.set_context_tracks(vec![track(1), track(2)]);
        pm.queue_next(track(10));

        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(10));
        assert!(pm.advance_next(false));
        assert_eq!(pm.current_track().map(|t| t.id), Some(2));

        let outcome = pm.advance(AdvanceReason::Natural);
        assert_eq!(outcome, AdvanceOutcome::Moved);
        assert_eq!(pm.current_track().map(|t| t.id), Some(1));
    }

    /// Loop state is unavailable when empty and becomes toggleable once tracks exist.
    #[test]
    fn loop_state_unavailable_until_tracks_exist() {
        let mut pm = PlaybackManager::new();
        assert_eq!(pm.loop_state(), LoopState::Unavailable);
        assert_eq!(pm.toggle_loop_state(), LoopState::Unavailable);

        pm.queue_next(track(1));
        assert_eq!(pm.loop_state(), LoopState::NotLooping);
        assert_eq!(pm.toggle_loop_state(), LoopState::LoopingTrack);
    }
}
