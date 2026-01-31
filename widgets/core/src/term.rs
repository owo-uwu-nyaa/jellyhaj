
use crate::{
    JellyhajWidget, JellyhajWidgetExt,
    async_task::{IdWrapper, TaskSubmitter},
};
use color_eyre::{Report, Result};
use ratatui::{CompletedFrame, Terminal};

pub trait TerminalExt {
    fn draw_widget<W: JellyhajWidget>(
        &mut self,
        widget: &mut W,
        task: TaskSubmitter<W::Action, IdWrapper>,
    ) -> Result<CompletedFrame<'_>>;
}

impl<B: ratatui::backend::Backend> TerminalExt for Terminal<B>
where
    B::Error: Send + Sync + 'static,
{
    fn draw_widget<W: JellyhajWidget>(
        &mut self,
        widget: &mut W,
        task: TaskSubmitter<W::Action, IdWrapper>,
    ) -> Result<CompletedFrame<'_>> {
        let mut err: Option<Report> = None;
        let err_ref = &mut err;
        let render_res = self.draw(move |frame| {
            *err_ref = widget
                .render_fallible(frame.area(), frame.buffer_mut(), task)
                .err();
            if err_ref.is_some() {
                frame.buffer_mut().reset();
            }
        });
        if let Some(e) = err {
            Err(e)
        } else {
            render_res.map_err(|e| e.into())
        }
    }
}
