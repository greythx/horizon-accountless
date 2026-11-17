//! Starting the game for Play: the command, the unsigned-developer mark and the instance's
//! launch. No accounts exist in this fork; there is no sign-in stage.
use super::*;

impl Launcher {
    pub(super) fn start_play(&mut self) {
        if let Some(tag) = self.starting_client.take() {
            let instance_id = self.starting_instance.take();
            let mut cmd = backend::client_command(&tag);
            if horizon_launcher::developer_offline::enabled(&tag) {
                if let Err(error) = horizon_launcher::developer_offline::require_capability(&tag) {
                    self.client_error = Some(error);
                    self.starting_pack = None;
                    self.joining = None;
                    self.quick_play = None;
                    return;
                }
                horizon_launcher::developer_offline::mark(&mut cmd);
                self.client_error = Some(tr(horizon_launcher::developer_offline::NOTICE).into());
            }
            // A pack's Play button plays that pack; Play plays the selected instance.
            let (profile, run) = match self.starting_pack.take() {
                Some(work) => {
                    let id = instances::list()
                        .into_iter()
                        .find(|i| i.work == work)
                        .map(|i| i.id);
                    let applied = horizon_launcher::modpacks::apply_profile(&mut cmd, &work);
                    (applied, id)
                }
                None => {
                    let Some(instance) = instances::list()
                        .into_iter()
                        .find(|i| Some(&i.id) == instance_id.as_ref())
                    else {
                        self.client_error = Some(
                            tr("This instance no longer exists. Choose another instance to play.")
                                .into(),
                        );
                        self.joining = None;
                        self.quick_play = None;
                        return;
                    };
                    (instances::apply(&mut cmd, &instance), Some(instance.id))
                }
            };
            if let Err(e) = profile {
                self.client_error = Some(e);
                return;
            }
            if let Some((flag, value)) = self.quick_play.take() {
                cmd.arg(flag).arg(value);
            }
            backend::append_game_args(&mut cmd);
            // Last, around every argument: the instance's pre-launch command,
            // wrapper and variables.
            let instance = run
                .as_ref()
                .and_then(|id| instances::list().into_iter().find(|i| &i.id == id));
            self.client_vars = instance
                .as_ref()
                .map(|i| i.vars(cmd.get_program()))
                .unwrap_or_default();
            cmd.envs(self.client_vars.iter().map(|(k, v)| (k, v)));
            let launch = instance.map(|i| i.launch).unwrap_or_default();
            self.starting_client = Some(tag);
            jar_launch::client(cmd, launch, run, self.tx.clone());
        }
    }
}
