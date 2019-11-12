use crate::config::Config;
use crate::git_interactive::GitInteractive;
use crate::input::{Input, InputHandler};
use crate::process::{
	ExitStatus,
	HandleInputResult,
	HandleInputResultBuilder,
	ProcessModule,
	ProcessResult,
	ProcessResultBuilder,
	State,
};
use crate::view::View;
use crate::window::Window;
use cmd_lib::run_cmd;

enum ExternalEditorState {
	Active,
	Error,
	Finish,
}

pub struct ExternalEditor<'e> {
	state: ExternalEditorState,
	config: &'e Config,
}

impl<'e> ProcessModule for ExternalEditor<'e> {
	fn activate(&mut self, _state: State, _git_interactive: &GitInteractive) {
		self.state = ExternalEditorState::Active;
	}

	fn process(&mut self, git_interactive: &mut GitInteractive, _view: &View) -> ProcessResult {
		match self.state {
			ExternalEditorState::Active => self.process_active(git_interactive),
			ExternalEditorState::Error => self.process_error(git_interactive),
			ExternalEditorState::Finish => self.process_finish(git_interactive),
		}
	}

	fn handle_input(
		&mut self,
		input_handler: &InputHandler,
		_git_interactive: &mut GitInteractive,
		_view: &View,
	) -> HandleInputResult
	{
		match self.state {
			ExternalEditorState::Active => self.handle_input_active(input_handler),
			_ => HandleInputResult::new(Input::Other),
		}
	}

	fn render(&self, _view: &View, _git_interactive: &GitInteractive) {}
}

impl<'e> ExternalEditor<'e> {
	pub fn new(config: &'e Config) -> Self {
		Self {
			config,
			state: ExternalEditorState::Active,
		}
	}

	pub fn run_editor(&mut self, git_interactive: &GitInteractive) -> Result<(), String> {
		git_interactive.write_file()?;
		let filepath = git_interactive.get_filepath();
		let callback = || -> Result<(), String> {
			run_cmd!("{} \"{}\"", self.config.editor.to_string_lossy(), filepath.as_os_str().to_string_lossy())
				.map_err(|e| {
					format!(
						"Unable to run editor ({} \"{}\"):\n{}",
						self.config.editor.to_string_lossy(),
						filepath.as_os_str().to_string_lossy(),
						e.to_string()
					)
				})
		};
		return Window::leave_temporarily(callback);
	}

	fn process_active(&mut self, git_interactive: &GitInteractive) -> ProcessResult {
		let mut result = ProcessResultBuilder::new();
		if let Err(e) = self.run_editor(git_interactive) {
			result = result.error(e.as_str(), State::ExternalEditor);
			self.state = ExternalEditorState::Error;
		}
		else {
			self.state = ExternalEditorState::Finish;
		}
		result.build()
	}

	fn process_finish(&mut self, git_interactive: &mut GitInteractive) -> ProcessResult {
		let mut result = ProcessResultBuilder::new();
		if let Err(e) = git_interactive.reload_file(self.config.comment_char.as_str()) {
			result = result.error(e.as_str(), State::ExternalEditor);
			self.state = ExternalEditorState::Error;
		}
		else if git_interactive.get_lines().is_empty() {
			result = result.error("Rebase TODO list is empty", State::ExternalEditor);
			self.state = ExternalEditorState::Error;
		}
		else {
			result = result.state(State::List(false));
		}
		result.build()
	}

	fn process_error(&self, git_interactive: &GitInteractive) -> ProcessResult {
		let mut result = ProcessResultBuilder::new().state(State::Exiting);

		if git_interactive.get_lines().is_empty() {
			result = result.exit_status(ExitStatus::Good);
		}
		else {
			result = result.exit_status(ExitStatus::StateError);
		}
		result.build()
	}

	pub fn handle_input_active(&self, input_handler: &InputHandler) -> HandleInputResult {
		let input = input_handler.get_input();
		let mut result = HandleInputResultBuilder::new(input);
		match input {
			Input::Resize => {},
			_ => {
				result = result.state(State::List(false));
			},
		}
		result.build()
	}
}
