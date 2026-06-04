use super::*;

mod command_research_foundation;
mod command_research_round17_to34;
mod command_research_round35_to52;
mod command_research_round55_to68;
mod command_research_round71_to78;

impl TyphooNApp {
    pub(super) fn handle_research_window_command(&mut self, cmd_upper: &String) -> bool {
        self.handle_research_foundation_command(cmd_upper)
            || self.handle_research_round17_to34_command(cmd_upper)
            || self.handle_research_round35_to52_command(cmd_upper)
            || self.handle_research_round55_to68_command(cmd_upper)
            || self.handle_research_round71_to78_command(cmd_upper)
    }
}
