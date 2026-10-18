//! Per-decode shared allocation/count budget, isolated to the calling thread.
//! RAII restores an outer budget on nested calls and on error/panic unwinding.
use std::cell::RefCell;

pub(crate) const MAX_SCENE_STORAGE_BYTES: usize = crate::resource_limits::MAX_SCENE_STORAGE_BYTES;

#[derive(Clone, Copy)]
pub(crate) enum Channel {
    Positions,
    Normals,
    Masks,
    Colors,
    Triangles,
    LayerVectors,
    LayerTriangles,
    MorphPositions,
    MorphTriangles,
    Materials,
    SurfaceFaces,
    UvFaces,
    UvCoordinates,
}

impl Channel {
    fn index(self) -> usize {
        self as usize
    }
    fn limit(self) -> usize {
        match self {
            Self::Triangles
            | Self::LayerTriangles
            | Self::MorphTriangles
            | Self::SurfaceFaces
            | Self::UvFaces => crate::MAX_TRIANGLES,
            Self::UvCoordinates => crate::MAX_UV_COORDINATES,
            _ => crate::MAX_VERTICES,
        }
    }
}

#[derive(Default)]
struct Budget {
    counts: [usize; 13],
    bytes: usize,
    layer_bytes: usize,
    morph_bytes: usize,
}

thread_local! {
    static BUDGET: RefCell<Option<Budget>> = const { RefCell::new(None) };
}

pub(crate) struct DecodeScope(Option<Budget>, std::marker::PhantomData<std::rc::Rc<()>>);

impl DecodeScope {
    pub(crate) fn enter() -> Self {
        Self(
            BUDGET.with(|slot| slot.replace(Some(Budget::default()))),
            std::marker::PhantomData,
        )
    }
}

impl Drop for DecodeScope {
    fn drop(&mut self) {
        BUDGET.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}

pub(crate) fn element(channel: Channel) -> Result<(), String> {
    BUDGET.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(budget) = slot.as_mut() else {
            return Ok(());
        };
        let count = &mut budget.counts[channel.index()];
        if *count >= channel.limit() {
            return Err("Scene aggregate channel allocation limit exceeded".into());
        }
        *count += 1;
        Ok(())
    })
}

pub(crate) fn remaining(channel: Channel) -> usize {
    BUDGET.with(|slot| {
        slot.borrow().as_ref().map_or(channel.limit(), |budget| {
            channel
                .limit()
                .saturating_sub(budget.counts[channel.index()])
        })
    })
}

pub(crate) fn capacity(channel: Channel, bytes: usize) -> Result<(), String> {
    BUDGET.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(budget) = slot.as_mut() else {
            return Ok(());
        };
        let total = budget
            .bytes
            .checked_add(bytes)
            .ok_or("Scene decode capacity overflow")?;
        if total > MAX_SCENE_STORAGE_BYTES {
            return Err("Scene decoded array capacity exceeds the scene allocation budget".into());
        }
        let category = match channel {
            Channel::LayerVectors | Channel::LayerTriangles => {
                Some((&mut budget.layer_bytes, crate::MAX_LAYER_STORAGE_BYTES))
            }
            Channel::MorphPositions | Channel::MorphTriangles => {
                Some((&mut budget.morph_bytes, crate::MAX_MORPH_STORAGE_BYTES))
            }
            _ => None,
        };
        if let Some((used, limit)) = category {
            let next = used
                .checked_add(bytes)
                .ok_or("Scene metadata capacity overflow")?;
            if next > limit {
                return Err("Scene aggregate layer/Morph capacity exceeds 64 MiB".into());
            }
            *used = next;
        }
        budget.bytes = total;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_decode_restores_outer_budget_and_error_cleanup_is_exact() {
        let _outer = DecodeScope::enter();
        capacity(Channel::Positions, MAX_SCENE_STORAGE_BYTES).unwrap();
        {
            let _inner = DecodeScope::enter();
            capacity(Channel::MorphPositions, 12).unwrap();
            assert!(capacity(Channel::Positions, MAX_SCENE_STORAGE_BYTES).is_err());
        }
        assert!(capacity(Channel::Positions, 1).is_err());
    }
    #[test]
    fn panic_unwinding_restores_outer_budget() {
        let _outer = DecodeScope::enter();
        capacity(Channel::Positions, MAX_SCENE_STORAGE_BYTES).unwrap();
        let result = std::panic::catch_unwind(|| {
            let _inner = DecodeScope::enter();
            capacity(Channel::Positions, 1).unwrap();
            panic!("intentional decoder scope unwind");
        });
        assert!(result.is_err());
        assert!(capacity(Channel::Positions, 1).is_err());
    }
    #[test]
    fn decode_budget_is_isolated_between_native_threads() {
        let _scope = DecodeScope::enter();
        capacity(Channel::Positions, MAX_SCENE_STORAGE_BYTES).unwrap();
        std::thread::spawn(|| {
            let _scope = DecodeScope::enter();
            capacity(Channel::Positions, 1).unwrap();
        })
        .join()
        .unwrap();
        assert!(capacity(Channel::Positions, 1).is_err());
    }
}
