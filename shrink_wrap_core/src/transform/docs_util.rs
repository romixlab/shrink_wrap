use crate::ast::docs::Docs;
use crate::ast::object_size::ObjectSize;

pub(crate) fn add_notes(docs: &mut Docs, size_assumption: Option<ObjectSize>, is_enum: bool) {
    match size_assumption {
        Some(ObjectSize::Unsized) | None => {
            docs.push_str("NOTE: This type is evolvable with backwards and forwards");
            docs.push_str("compatibility (reserved bits can be used, Option<T>, String or Vec<T> fields can be added to the back).");
        }
        Some(ObjectSize::UnsizedFinalStructure)
        | Some(ObjectSize::Sized { .. })
        | Some(ObjectSize::SelfDescribing) => {
            docs.push_str(
                "NOTE: Size or structure of this type can no longer be changed without breaking compatibility,",
            );
            docs.push_str("only reserved bits can still be used to carry new information.");
        }
    }
    if is_enum {
        docs.push_str("Enum variants can be added if there is space left and if code already in use can handle them.")
    }
}
