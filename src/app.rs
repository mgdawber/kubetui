use std::{error::Error, process::Command};
use tui::widgets::ListState;

#[derive(PartialEq)]
pub enum AppState {
    MainMenu,
    NamespaceSelection,
    ContextSelection,
    ExecPodSelection,
    PodSelection,
    CopyPodNameInput,
    Message,
    ShowOutput,
}

pub struct App {
    pub state: AppState,

    pub commands: Vec<String>,
    pub list_state: ListState,

    pub namespaces: Vec<String>,
    pub namespace_list_state: ListState,

    pub contexts: Vec<String>,
    pub context_list_state: ListState,

    pub pods: Vec<String>,
    pub pod_list_state: ListState,

    pub selected_namespace: Option<String>,
    pub selected_context: Option<String>,
    pub selected_pod: Option<String>,

    pub default_namespace: String,

    pub input: String,
    pub new_pod_name: String,

    pub message: String,
    pub output: String,

    pub last_main_menu_index: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        let default_context = Self::get_current_context();

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        App {
            state: AppState::MainMenu,

            commands: vec![
                "Choose Context".to_string(),
                "Choose Namespace".to_string(),
                "Pods".to_string(),
                "Copy Pod".to_string(),
            ],
            list_state,

            namespaces: Vec::new(),
            namespace_list_state: ListState::default(),

            contexts: Vec::new(),
            context_list_state: ListState::default(),

            pods: Vec::new(),
            pod_list_state: ListState::default(),

            selected_namespace: None,
            selected_context: default_context,
            selected_pod: None,

            default_namespace: "default".to_string(),

            input: String::new(),
            new_pod_name: String::new(),
            message: String::new(),
            output: String::new(),

            last_main_menu_index: None,
        }
    }

    fn get_current_context() -> Option<String> {
        let output = Command::new("kubectl")
            .args(&["config", "current-context"])
            .output()
            .ok()?;

        if output.status.success() {
            let ctx = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ctx.is_empty() {
                Some(ctx)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn current_namespace(&self) -> String {
        self.selected_namespace
            .clone()
            .unwrap_or_else(|| self.default_namespace.clone())
    }

    pub fn load_namespaces(&mut self) -> Result<(), Box<dyn Error>> {
        let output = Command::new("kubectl")
            .args(&["get", "namespaces", "-o=jsonpath='{.items[*].metadata.name}'"])
            .output()?;

        if output.status.success() {
            let ns_output = String::from_utf8_lossy(&output.stdout)
                .trim_matches('\'')
                .to_string();
            self.namespaces = ns_output
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            self.namespace_list_state.select(Some(0));
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to get namespaces: {}", error_msg).into())
        }
    }

    pub fn load_contexts(&mut self) -> Result<(), Box<dyn Error>> {
        let output = Command::new("kubectl")
            .args(&["config", "get-contexts", "-o=name"])
            .output()?;

        if output.status.success() {
            self.contexts = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect();
            self.context_list_state.select(Some(0));
            Ok(())
        } else {
            Err("Failed to load contexts".into())
        }
    }

    pub fn load_pods(&mut self) -> Result<(), Box<dyn Error>> {
        let namespace = self.current_namespace();
        let output = Command::new("kubectl")
            .args(&["get", "pods", "-n", &namespace, "-o=jsonpath='{.items[*].metadata.name}'"])
            .output()?;

        if output.status.success() {
            let pod_output = String::from_utf8_lossy(&output.stdout)
                .trim_matches('\'')
                .to_string();
            self.pods = pod_output
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            self.pod_list_state.select(Some(0));
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to get pods: {}", error_msg).into())
        }
    }

    pub fn switch_context(&mut self, context: &str) -> Result<(), Box<dyn Error>> {
        let status = Command::new("kubectl")
            .args(&["config", "use-context", context])
            .status()?;

        if status.success() {
            self.selected_context = Some(context.to_string());
            Ok(())
        } else {
            Err("Failed to switch context".into())
        }
    }

    pub fn execute_kubectl(&mut self, args: &[&str]) -> Result<(), Box<dyn Error>> {
        let output = Command::new("kubectl")
            .args(args)
            .output()?;

        self.output = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };
        Ok(())
    }

    pub fn copy_pod(&mut self, original_pod: &str, new_pod_name: &str) -> Result<(), Box<dyn Error>> {
        let namespace = self.current_namespace();
        let output = Command::new("kubectl")
            .args(&[
                "debug",
                "-it",
                "-n",
                &namespace,
                original_pod,
                "--copy-to",
                new_pod_name,
                "--container=worker",
                "--",
                "bash",
            ])
            .output()?;

        if output.status.success() {
            self.output = String::from_utf8_lossy(&output.stdout).to_string();
        } else {
            self.output = String::from_utf8_lossy(&output.stderr).to_string();
        }
        self.state = AppState::ShowOutput;
        Ok(())
    }

    pub fn exec_pod(&mut self, pod: &str) -> Result<(), Box<dyn Error>> {
        let namespace = self.current_namespace();
        let output = Command::new("kubectl")
            .args(&["exec", "-it", "-n", &namespace, pod, "--", "bash"])
            .output()?;

        if output.status.success() {
            self.output = String::from_utf8_lossy(&output.stdout).to_string();
        } else {
            self.output = String::from_utf8_lossy(&output.stderr).to_string();
        }
        self.state = AppState::ShowOutput;
        Ok(())
    }
}
