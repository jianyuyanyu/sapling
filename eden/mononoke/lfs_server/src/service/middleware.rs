/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::pin::Pin;

use cached_config::ConfigHandle;
use fbinit::FacebookInit;
use futures::future::FutureExt;
use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::FromState;
use gotham::state::State;
use gotham_derive::NewMiddleware;
use gotham_ext::error::HttpError;
use gotham_ext::middleware::MetadataState;
use gotham_ext::response::build_error_response;
use hyper::Uri;
use ods_counters::OdsCounterManager;
use rate_limiting::LoadShedResult;
use scuba_ext::MononokeScubaSampleBuilder;

use super::error_formatter::LfsErrorFormatter;
use crate::config::ServerConfig;

// NOTE: Our Throttling middleware is implemented as Gotham middleware for 3 reasons:
// - It needs to replace responses.
// - It needs to do asynchronously.
// - It only needs to run if we're going to serve a request.

#[derive(Clone, NewMiddleware)]
pub struct ThrottleMiddleware {
    fb: FacebookInit,
    handle: ConfigHandle<ServerConfig>,
    scuba: MononokeScubaSampleBuilder,
}

impl ThrottleMiddleware {
    pub fn new(
        fb: FacebookInit,
        handle: ConfigHandle<ServerConfig>,
        scuba: MononokeScubaSampleBuilder,
    ) -> Self {
        Self { fb, handle, scuba }
    }
}

impl Middleware for ThrottleMiddleware {
    fn call<Chain>(mut self, state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>>,
    {
        if let Some(uri) = Uri::try_borrow_from(&state) {
            if uri.path() == "/health_check" {
                return chain(state);
            }
        }

        let (identities, main_client_id) = match state.try_borrow::<MetadataState>() {
            Some(metadata_state) => {
                let client_id = metadata_state
                    .metadata()
                    .client_request_info()
                    .and_then(|info| info.main_id.clone());
                (Some(metadata_state.metadata().identities()), client_id)
            }
            None => (None, None),
        };

        for limit in self.handle.get().loadshedding_limits().iter() {
            if let LoadShedResult::Fail(err) = limit.should_load_shed(
                self.fb,
                identities,
                main_client_id.as_deref(),
                &mut self.scuba,
                OdsCounterManager::new(self.fb),
            ) {
                let err = HttpError::e429(err);

                let res =
                    async move { build_error_response(err, state, &LfsErrorFormatter) }.boxed();

                return res;
            }
        }

        chain(state)
    }
}
