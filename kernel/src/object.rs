use core::num::NonZeroU64;
use core::marker::PhantomData;

use alloc_collections::btree_map::BTreeMap;
use interface::{SysResult, SysError};

use crate::mem::MemoryExhausted;
use crate::mem::kalloc::GlobalAlloc;
use crate::mem::page::PageCtx;
use crate::sync::{Arc, Mutex};
use crate::task::{TaskId, TaskMap};
use crate::util::EarlyInit;

#[derive(Debug)]
pub enum ObjectKind {
    PageCtx(PageCtx),
}

pub trait ObjectKindT {
    fn wrap(self) -> ObjectKind;
    fn as_ref(kind: &ObjectKind) -> SysResult<&Self>;
}

impl ObjectKindT for PageCtx {
    fn wrap(self) -> ObjectKind {
        ObjectKind::PageCtx(self)
    }

    fn as_ref(kind: &ObjectKind) -> SysResult<&Self> {
        if let ObjectKind::PageCtx(ref a) = kind {
            Ok(a)
        } else {
            Err(SysError::WrongObjectKind)
        }
    }
}

#[derive(Debug)]
pub struct Object {
    kind: ObjectKind,
}

pub type DynObjectRef = Arc<Object>;

impl Object {
    pub fn new(kind: ObjectKind) -> Result<DynObjectRef, MemoryExhausted> {
        Arc::new(Object { kind })
    }

    pub fn downcast<T: ObjectKindT>(self: DynObjectRef) -> SysResult<ObjectRef<T>> {
        ObjectRef::from_dyn(self)
    }
}

#[derive(Debug, Clone)]
pub struct ObjectRef<T>{
    ref_: DynObjectRef,
    phantom: PhantomData<T>,
}

impl<T: ObjectKindT> ObjectRef<T> {
    pub fn new(obj: T) -> Result<Self, MemoryExhausted> {
        Ok(ObjectRef {
            ref_: Object::new(ObjectKindT::wrap(obj))?,
            phantom: PhantomData,
        })
    }

    pub fn from_dyn(ref_: DynObjectRef) -> SysResult<Self> {
        // type check:
        T::as_ref(&ref_.kind)?;

        Ok(ObjectRef {
            ref_,
            phantom: PhantomData,
        })
    }

    pub fn as_dyn(&self) -> DynObjectRef {
        self.ref_.clone()
    }

    pub fn object(&self) -> &T {
        ObjectKindT::as_ref(&self.ref_.kind)
            .expect("unexpected object kind")
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
pub struct Handle(pub NonZeroU64);

impl Handle {
    pub fn from_u64(u: u64) -> Option<Handle> {
        NonZeroU64::new(u).map(Handle)
    }

    pub fn into_u64(&self) -> u64 {
        self.0.get()
    }
}

static TASK_HANDLES: TaskMap<BTreeMap<Handle, DynObjectRef, GlobalAlloc>> = TaskMap::new();

pub fn init() {
    EarlyInit::set(&TASK_HANDLES, Mutex::new(BTreeMap::new()));
}

pub fn put(task_id: TaskId, object: DynObjectRef) -> SysResult<Handle> {
    let mut task_handles = TASK_HANDLES.lock();

    let mut handles = match task_handles.get_mut(&task_id) {
        Some(handles) => handles,
        None => {
            let handles = BTreeMap::new();

            task_handles.insert(task_id, handles)
                .map_err(|_| SysError::MemoryExhausted)?;

            task_handles.get_mut(&task_id).expect("should never fail")
        }
    };

    let new_id = handles.keys().rev().nth(0)
        .map(|h| Handle(
            NonZeroU64::new(h.0.get() + 1)
                .expect("handle wrap around")))
        .unwrap_or(Handle(NonZeroU64::new(1).expect("impossible")));

    handles.insert(new_id.clone(), object)
        .map_err(|_| SysError::MemoryExhausted)?;

    Ok(new_id)
}

pub fn get(task_id: TaskId, handle: Handle) -> Option<DynObjectRef> {
    TASK_HANDLES.lock().get(&task_id)?.get(&handle).cloned()
}

pub fn release(task_id: TaskId, handle: Handle) -> Result<DynObjectRef, ()> {
    TASK_HANDLES.lock().get_mut(&task_id)
        .and_then(|mut map| map.remove(&handle))
        .ok_or(())
}

pub fn drop_all_for_task(task_id: TaskId) {
    TASK_HANDLES.lock().remove(&task_id);
}
