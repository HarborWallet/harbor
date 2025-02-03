use iced::Element;

use crate::components::{h_button, h_header, SvgIcon};
use crate::components::{Toast, ToastStatus};
use crate::Message;
use iced::widget::column;

// This is just for testing purposes
pub fn debug_stuff() -> Element<'static, Message> {
    let header = h_header(
        "Debug Stuff",
        "If you're seeing this you're in dev mode or possibly in a dream.",
    );

    let add_good_toast_button =
        h_button("Nice!", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Hello".to_string(),
            body: Some("This is a toast".to_string()),
            status: ToastStatus::Good,
        }));

    let add_error_toast_button =
        h_button("Error Toast", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Error".to_string(),
            body: Some("This is a toast".to_string()),
            status: ToastStatus::Bad,
        }));

    let test_confirm_modal_button = h_button("Test Confirm Modal", SvgIcon::Shield, false)
        .on_press(Message::SetConfirmModal(Some(
            crate::components::ConfirmModalState {
                title: "Test Modal".to_string(),
                description:
                    "This is a test of the confirm modal. Are you sure you want to proceed?"
                        .to_string(),
                confirm_action: Box::new(Message::Batch(vec![
                    Message::AddToast(Toast {
                        title: "You confirmed!".to_string(),
                        body: None,
                        status: ToastStatus::Good,
                    }),
                    Message::SetConfirmModal(None),
                ])),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Confirm".to_string(),
            },
        )));

    let column = column![
        header,
        add_good_toast_button,
        add_error_toast_button,
        test_confirm_modal_button
    ];

    column.spacing(48).into()
}
