use super::*;
use crate::state::ids::QueryId;

impl KeybaseBackend {
    pub(super) fn schedule_user_search(
        &mut self,
        query_id: QueryId,
        query: String,
    ) -> Result<Vec<BackendEvent>, BackendError> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Ok(vec![BackendEvent::UserSearchResults {
                query_id,
                results: Vec::new(),
            }]);
        }
        self.ensure_listener_started();
        let sender = self.inbound_sender.clone().ok_or_else(|| {
            BackendError::OperationFailed("Keybase is unavailable. Please try again.".into())
        })?;
        let command = BackendCommand::SearchUsers {
            query_id: query_id.clone(),
            query: query.clone(),
        };
        let outcome = BackendTaskOutcome::new(&sender, command.clone());
        let generation = Arc::clone(&self.user_search_generation);
        let current = generation.fetch_add(1, Ordering::Relaxed) + 1;
        task_runtime::spawn_task(TaskPriority::Interactive, None, move || {
            let mut outcome = outcome;
            // Superseded work waiting for a worker must not start another RPC.
            if generation.load(Ordering::Relaxed) != current {
                outcome.complete();
                return;
            }
            let result = (|| -> io::Result<Vec<UserSummary>> {
                let path = socket_path()
                    .ok_or_else(|| io::Error::other("Keybase is unavailable. Please try again."))?;
                let runtime = Builder::new_current_thread().enable_all().build()?;
                runtime.block_on(async {
                    let transport = FramedMsgpackTransport::connect(&path).await?;
                    let mut client = KeybaseRpcClient::new(transport);
                    let response = tokio::time::timeout(
                        Duration::from_secs(15),
                        call_user_search(&mut client, &query),
                    )
                    .await
                    .map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::TimedOut,
                            "Keybase search timed out. Please try again.",
                        )
                    })??;
                    Ok(parse_user_search_results(&response))
                })
            })();
            let event = match result {
                Ok(results) => BackendEvent::UserSearchResults { query_id, results },
                Err(error) => BackendEvent::CommandFailed {
                    command: Box::new(command),
                    error: error.to_string(),
                },
            };
            let _ = sender.send(event);
            outcome.complete();
        });
        Ok(Vec::new())
    }
}
