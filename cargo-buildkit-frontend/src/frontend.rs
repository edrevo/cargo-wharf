use failure::{Error, ResultExt};
use futures::prelude::*;

use buildkit_frontend::{Bridge, Frontend, FrontendOutput, Options};

use crate::config::Config;
use crate::debug::DebugOperation;
use crate::graph::BuildGraph;
use crate::plan::RawBuildPlan;
use crate::query::GraphQuery;

pub struct CargoFrontend;

impl Frontend for CargoFrontend {
    type RunFuture = impl Future<Output = Result<FrontendOutput, Error>>;

    fn run(self, mut bridge: Bridge, options: Options) -> Self::RunFuture {
        async move {
            let mut debug = DebugOperation::new();

            let config = {
                Config::analyse(&mut bridge)
                    .await
                    .context("Unable to analyse config")?
            };

            debug.maybe(&options, &config);

            let plan = {
                RawBuildPlan::evaluate(&mut bridge, &config)
                    .await
                    .context("Unable to evaluate the Cargo build plan")?
            };

            debug.maybe(&options, &plan);

            let graph: BuildGraph = plan.into();
            let query = GraphQuery::new(&graph, &config);

            debug.maybe(&options, &graph);
            debug.maybe(&options, &query.definition());

            if options.has("debug") {
                return Ok(FrontendOutput::with_ref(
                    bridge
                        .solve(debug.terminal())
                        .await
                        .context("Unable to write debug output")?,
                ));
            }

            Ok(FrontendOutput::with_spec_and_ref(
                query.image_spec().context("Unable to build image spec")?,
                query
                    .solve(&mut bridge)
                    .await
                    .context("Unable to build the crate")?,
            ))
        }
    }
}
