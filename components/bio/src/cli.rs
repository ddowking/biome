pub mod gateway_util;
pub mod bio;

use crate::{cli::bio::{origin::Rbac,
                       pkg::{ExportCommand,
                             PkgExec},
                       studio::Studio,
                       sup::{BioSup,
                             SupRun},
                       svc::{BulkLoad as SvcBulkLoad,
                             Load as SvcLoad,
                             Update as SvcUpdate},
                       util::CACHE_KEY_PATH_DEFAULT,
                       Bio},
            command::studio};
use clap::{App,
           AppSettings,
           Arg,
           ArgMatches};
use biome_common::{cli::{file_into_idents,
                           is_toml_file,
                           BINLINK_DIR_ENVVAR,
                           DEFAULT_BINLINK_DIR,
                           PACKAGE_TARGET_ENVVAR},
                     FeatureFlag};
use biome_core::{crypto::CACHE_KEY_PATH_ENV_VAR,
                   env::Config,
                   origin::Origin,
                   os::process::ShutdownTimeout,
                   package::{Identifiable,
                             PackageIdent,
                             PackageTarget},
                   service::ServiceGroup,
                   ChannelIdent};
use std::{path::Path,
          result,
          str::FromStr};
use structopt::StructOpt;
use url::Url;

/// Process exit code from Supervisor which indicates to Launcher that the Supervisor
/// ran to completion with a successful result. The Launcher should not attempt to restart
/// the Supervisor and should exit immediately with a successful exit code.
pub const OK_NO_RETRY_EXCODE: i32 = 84;
pub const AFTER_HELP: &str =
    "\nALIASES:\n    apply      Alias for: 'config apply'\n    install    Alias for: 'pkg \
     install'\n    run        Alias for: 'sup run'\n    setup      Alias for: 'cli setup'\n    \
     start      Alias for: 'svc start'\n    stop       Alias for: 'svc stop'\n    term       \
     Alias for: 'sup term'\n";

pub fn get(feature_flags: FeatureFlag) -> App<'static, 'static> {
    if feature_flags.contains(FeatureFlag::STRUCTOPT_CLI) {
        return Bio::clap();
    }

    let alias_apply = sub_config_apply().about("Alias for 'config apply'")
                                        .aliases(&["ap", "app", "appl"])
                                        .setting(AppSettings::Hidden);
    let alias_install =
        sub_pkg_install(feature_flags).about("Alias for 'pkg install'")
                                      .aliases(&["i", "in", "ins", "inst", "insta", "instal"])
                                      .setting(AppSettings::Hidden);
    let alias_run = SupRun::clap().about("Alias for 'sup run'")
                                  .setting(AppSettings::Hidden);
    let alias_setup = sub_cli_setup().about("Alias for 'cli setup'")
                                     .aliases(&["set", "setu"])
                                     .setting(AppSettings::Hidden);
    let alias_start = sub_svc_start().about("Alias for 'svc start'")
                                     .aliases(&["sta", "star"])
                                     .setting(AppSettings::Hidden);
    let alias_stop = sub_svc_stop().about("Alias for 'svc stop'")
                                   .aliases(&["sto"])
                                   .setting(AppSettings::Hidden);

    clap_app!(bio =>
        (about: "\"A Biome is the natural environment for your services\" - Alan Turing")
        (version: super::VERSION)
        (author: "\nThe Biome Maintainers <humans@biome.sh>\n")
        (@setting GlobalVersion)
        (@setting ArgRequiredElseHelp)
        (@setting SubcommandRequiredElseHelp)
        (@subcommand license =>
            (about: "Commands relating to Biome license agreements")
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand accept =>
                (about: "Accept the Biome Binary Distribution Agreement without prompting"))
        )
        (@subcommand cli =>
            (about: "Commands relating to Biome runtime config")
            (aliases: &["cl"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (subcommand: sub_cli_setup().aliases(&["s", "se", "set", "setu"]))
            (subcommand: sub_cli_completers().aliases(&["c", "co", "com", "comp"]))
        )
        (@subcommand config =>
            (about: "Commands relating to a Service's runtime config")
            (aliases: &["co", "con", "conf", "confi"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (subcommand: sub_config_apply().aliases(&["ap", "app", "appl"]))
            (@subcommand show =>
                (about: "Displays the default configuration options for a service")
                (aliases: &["sh", "sho"])
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
                (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
                    "Address to a remote Supervisor's Control Gateway")
            )
        )
        (@subcommand file =>
            (about: "Commands relating to Biome files")
            (aliases: &["f", "fi", "fil"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand upload =>
                (about: "Uploads a file to be shared between members of a Service Group")
                (aliases: &["u", "up", "upl", "uplo", "uploa"])
                (@arg SERVICE_GROUP: +required +takes_value {valid_service_group}
                    "Target service group service.group[@organization] (ex: redis.default or foo.default@bazcorp)")
                (@arg VERSION_NUMBER: +required +takes_value
                    "A version number (positive integer) for this configuration (ex: 42)")
                (@arg FILE: +required +takes_value {file_exists} "Path to local file on disk")
                (@arg USER: -u --user +takes_value "Name of the user key")
                (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
                    "Address to a remote Supervisor's Control Gateway")
                (arg: arg_cache_key_path())
            )
        )
        (@subcommand bldr =>
            (about: "Commands relating to Biome Builder")
            (aliases: &["b", "bl", "bld"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand job =>
                (about: "Commands relating to Biome Builder jobs")
                (aliases: &["j", "jo"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand start =>
                    (about: "Schedule a build job or group of jobs")
                    (aliases: &["s", "st", "sta", "star"])
                    (@arg PKG_IDENT: +required +takes_value {valid_ident}
                        "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
                    (arg: arg_target())
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. \
                         (default: https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                    (@arg GROUP: -g --group "Schedule jobs for this package and all of its reverse \
                        dependencies")
                )
                (@subcommand cancel =>
                    (about: "Cancel a build job group and any in-progress builds")
                    (aliases: &["c", "ca", "can", "cance", "cancel"])
                    (@arg GROUP_ID: +required +takes_value
                        "The job group id that was returned from \"bio bldr job start\" \
                        (ex: 771100000000000000)")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg FORCE: -f --force
                     "Don't prompt for confirmation")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand promote =>
                    (about: "Promote packages from a completed build job to a specified channel")
                    (aliases: &["p", "pr", "pro", "prom", "promo", "promot"])
                    (@arg GROUP_ID: +required +takes_value
                        "The job group id that was returned from \"bio bldr job start\" \
                        (ex: 771100000000000000)")
                    (@arg CHANNEL: +takes_value +required "The target channel name")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "Limit the promotable packages to the specified origin")
                    (@arg INTERACTIVE: -i --interactive
                        "Allow editing the list of promotable packages")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand demote =>
                    (about: "Demote packages from a completed build job from a specified channel")
                    (aliases: &["d", "de", "dem", "demo", "demot"])
                    (@arg GROUP_ID: +required +takes_value
                        "The job group id that was returned from \"bio bldr job start\" \
                        (ex: 771100000000000000)")
                    (@arg CHANNEL: +takes_value +required "The name of the channel to demote from")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "Limit the demotable packages to the specified origin")
                    (@arg INTERACTIVE: -i --interactive
                        "Allow editing the list of demotable packages")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand status =>
                    (about: "Get the status of one or more job groups")
                    (aliases: &["stat", "statu"])
                    (@group status =>
                        (@attributes +required)
                        (@arg GROUP_ID: +takes_value
                            "The job group id that was returned from \"bio bldr job start\" \
                            (ex: 771100000000000000)")
                        (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                            "Show the status of recent job groups created in this origin \
                            (default: 10 most recent)")
                    )
                    (@arg LIMIT: -l --limit +takes_value {valid_numeric::<usize>}
                        "Limit how many job groups to retrieve, ordered by most recent \
                        (default: 10)")
                    (@arg SHOW_JOBS: -s --showjobs
                        "Show the status of all build jobs for a retrieved job group")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                )
            )
            (@subcommand channel =>
                (about: "Commands relating to Biome Builder channels")
                (aliases: &["c", "ch", "cha", "chan", "chann", "channe"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand promote =>
                    (about: "Atomically promotes all packages in channel")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg ORIGIN: -o --origin +required +takes_value {valid_origin}
                        "The origin for the channels. Default is from \
                        'HAB_ORIGIN' or cli.toml")
                    (@arg SOURCE_CHANNEL: +required +takes_value
                        "The channel from which all packages will be selected for promotion")
                    (@arg TARGET_CHANNEL: +required +takes_value
                        "The channel to which packages will be promoted")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")

                )
                (@subcommand demote =>
                    (about: "Atomically demotes selected packages in a target channel")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg ORIGIN: -o --origin +required +takes_value {valid_origin}
                        "The origin for the channels. Default is from \
                        'HAB_ORIGIN' or cli.toml")
                    (@arg SOURCE_CHANNEL: +required +takes_value
                        "The channel from which all packages will be selected for demotion")
                    (@arg TARGET_CHANNEL: +required +takes_value
                        "The channel selected packages will be removed from")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")

                )
                (@subcommand create =>
                    (about: "Creates a new channel")
                    (aliases: &["c", "cr", "cre", "crea", "creat"])
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg CHANNEL: +required + takes_value "The channel name")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "Sets the origin to which the channel will belong. Default is from \
                        'HAB_ORIGIN' or cli.toml")
                )
                (@subcommand destroy =>
                    (about: "Destroys a channel")
                    (aliases: &["d", "de", "des", "dest", "destr", "destro"])
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg CHANNEL: +required + takes_value "The channel name")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "Sets the origin to which the channel belongs. Default is from 'HAB_ORIGIN' \
                        or cli.toml")
                )
                (@subcommand list =>
                    (about: "Lists origin channels")
                    (aliases: &["l", "li", "lis"])
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg ORIGIN: +takes_value {valid_origin}
                        "The origin for which channels will be listed. Default is from 'HAB_ORIGIN' \
                        or cli.toml")
                )
            )
        )
        (@subcommand origin =>
            (about: "Commands relating to Biome Builder origins")
            (aliases: &["o", "or", "ori", "orig", "origi"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand create =>
                (about: "Creates a new Builder origin")
                (aliases: &["cre", "crea"])
                (@arg ORIGIN: +required +takes_value {valid_origin} "The origin to be created")
                (@arg BLDR_URL: -u --url +takes_value {valid_url}
                     "Specify an alternate Builder endpoint. If not specified, the value will \
                     be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                     https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand delete =>
                (about: "Removes an unused/empty origin")
                (aliases: &["del", "dele"])
                (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name")
                (@arg BLDR_URL: -u --url +takes_value {valid_url}
                     "Specify an alternate Builder endpoint. If not specified, the value will \
                     be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                     https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand transfer =>
                (about: "Transfers ownership of an origin to another member of that origin")
                (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name")
                (@arg BLDR_URL: -u --url +takes_value {valid_url}
                     "Specify an alternate Builder endpoint. If not specified, the value will \
                     be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                     https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                (@arg NEW_OWNER_ACCOUNT: +required +takes_value {non_empty} "The account name of the new origin owner")
            )
            (@subcommand depart =>
                (about: "Departs membership from selected origin")
                (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name")
                (@arg BLDR_URL: -u --url +takes_value {valid_url}
                     "Specify an alternate Builder endpoint. If not specified, the value will \
                     be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                     https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand info =>
                (about: "Displays general information about an origin")
                (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name to be queried")
                (@arg TO_JSON: -j --json "Output will be rendered in json")
                (@arg BLDR_URL: -u --url +takes_value {valid_url}
                     "Specify an alternate Builder endpoint. If not specified, the value will \
                     be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                     https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand invitations =>
                (about: "Manage origin member invitations")
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand accept =>
                     (about: "Accept an origin member invitation")
                     (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name the invitation applies to")
                     (@arg INVITATION_ID: +required +takes_value {valid_numeric::<u64>} "The id of the invitation to accept")
                     (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                     (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand ignore =>
                     (about: "Ignore an origin member invitation")
                     (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name the invitation applies to")
                     (@arg INVITATION_ID: +required +takes_value {valid_numeric::<u64>} "The id of the invitation to ignore")
                     (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                     (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand list =>
                     (about: "List origin invitations sent to your account")
                     (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                     (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand pending =>
                     (about: "List pending invitations for a particular origin. Requires that you are the origin owner")
                     (@arg ORIGIN: +required +takes_value {valid_origin} "The name of the origin you wish to list invitations for")
                     (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                     (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand rescind =>
                    (about: "Rescind an existing origin member invitation")
                    (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name the invitation applies to")
                     (@arg INVITATION_ID: +required +takes_value {valid_numeric::<u64>} "The id of the invitation to rescind")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
                (@subcommand send =>
                     (about: "Send an origin member invitation")
                     (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name the invitation applies to")
                     (@arg INVITEE_ACCOUNT: +required +takes_value {non_empty} "The account name to invite into the origin")
                     (@arg BLDR_URL: -u --url +takes_value {valid_url}
                          "Specify an alternate Builder endpoint. If not specified, the value will \
                          be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                          https://bldr.habitat.sh)")
                     (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
            )
            (@subcommand key =>
                (about: "Commands relating to Biome origin key maintenance")
                (aliases: &["k", "ke"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand download =>
                    (about: "Download origin key(s)")
                    (aliases: &["d", "do", "dow", "down", "downl", "downlo", "downloa"])
                    (arg: arg_cache_key_path())
                    (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name" )
                    (@arg REVISION: +takes_value "The origin key revision")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg WITH_SECRET: -s --secret
                        "Download origin private key instead of origin public key")
                    (@arg WITH_ENCRYPTION: -e --encryption
                        "Download public encryption key instead of origin public key")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder \
                        (required for downloading origin private keys)")
                )
                (@subcommand export =>
                    (about: "Outputs the latest origin key contents to stdout")
                    (aliases: &["e", "ex", "exp", "expo", "expor"])
                    (@arg ORIGIN: +required +takes_value {valid_origin} "The origin name")
                    (@arg KEY_TYPE: -t --type +takes_value {valid_key_type}
                        "Export either the 'public' or 'secret' key. The 'secret' key is the origin private key")
                    (arg: arg_cache_key_path())
                )
                (@subcommand generate =>
                    (about: "Generates a Biome origin key pair")
                    (aliases: &["g", "ge", "gen", "gene", "gener", "genera", "generat"])
                    (@arg ORIGIN: +takes_value {valid_origin} "The origin name")
                    (arg: arg_cache_key_path())

                )
                (@subcommand import =>
                    (about: "Reads a stdin stream containing a public or private origin key \
                        contents and writes the key to disk")
                    (aliases: &["i", "im", "imp", "impo", "impor"])
                    (arg: arg_cache_key_path())
                )
                (@subcommand upload =>
                    (@group upload =>
                        (@attributes +required)
                        (@arg ORIGIN : +takes_value {valid_origin} "The origin name")
                        (@arg PUBLIC_FILE: --pubfile +takes_value {file_exists}
                            "Path to a local public origin key file on disk")
                    )
                    (about: "Upload origin keys to Builder")
                    (aliases: &["u", "up", "upl", "uplo", "uploa"])
                    (arg: arg_cache_key_path())
                    (@arg WITH_SECRET: -s --secret conflicts_with[PUBLIC_FILE]
                        "Upload origin private key in addition to the public key")
                    (@arg SECRET_FILE: --secfile +takes_value {file_exists} conflicts_with[ORIGIN]
                        "Path to a local origin private key file on disk")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                )
            )
            (subcommand: Rbac::clap())
            (@subcommand secret =>
                (about: "Commands related to secret management")
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand upload =>
                    (about: "Create and upload a secret for your origin")
                    (@arg KEY_NAME: +required +takes_value
                        "The name of the variable key to be injected into the studio. \
                        Ex: KEY=\"some_value\"")
                    (@arg SECRET: +required +takes_value
                        "The contents of the variable to be injected into the studio")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "The origin for which the secret will be uploaded. Default is from \
                        'HAB_ORIGIN' or cli.toml")
                    (arg: arg_cache_key_path())
                )
                (@subcommand delete =>
                    (about: "Delete a secret for your origin")
                    (@arg KEY_NAME: +required +takes_value
                        "The name of the variable key to be injected into the studio")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "The origin for which the secret will be deleted. Default is from \
                        'HAB_ORIGIN' or cli.toml")
                )
                (@subcommand list =>
                    (about: "List all secrets for your origin")
                    (@arg BLDR_URL: -u --url +takes_value {valid_url}
                        "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
                    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                        "The origin for which secrets will be listed. Default is from 'HAB_ORIGIN' \
                        or cli.toml")
                )
            )
        )
        (@subcommand pkg =>
            (about: "Commands relating to Biome packages")
            (aliases: &["p", "pk", "package"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand binds =>
                (about: "Displays the binds for a service")
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
            )
            (@subcommand binlink =>
                (about: "Creates a binlink for a package binary in a common 'PATH' location")
                (aliases: &["bi", "bin", "binl", "binli", "binlin"])
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
                (@arg BINARY: +takes_value
                    "The command to binlink (ex: bash)")
                (@arg DEST_DIR: -d --dest +takes_value {non_empty} env(BINLINK_DIR_ENVVAR) default_value(DEFAULT_BINLINK_DIR)
                    "Sets the destination directory")
                (@arg FORCE: -f --force "Overwrite existing binlinks")
             )
            (subcommand: sub_pkg_build())
            (@subcommand config =>
                (about: "Displays the default configuration options for a service")
                (aliases: &["conf", "cfg"])
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
             )
            (subcommand: sub_pkg_download())
            (@subcommand env =>
                (about: "Prints the runtime environment of a specific installed package")
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
            )
            (subcommand: PkgExec::clap())
            (subcommand: ExportCommand::clap())
            (@subcommand hash =>
                (about: "Generates a blake2b hashsum from a target at any given filepath")
                (aliases: &["ha", "has"])
                (@arg SOURCE: +takes_value {file_exists} "A filepath of the target")
            )
            (subcommand: sub_pkg_install(feature_flags).aliases(
                &["i", "in", "ins", "inst", "insta", "instal"]))
            (@subcommand path =>
                (about: "Prints the path to a specific installed release of a package")
                (aliases: &["p", "pa", "pat"])
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
            )
            (@subcommand list =>
                (about: "List all versions of installed packages")
                (aliases: &["li"])
                (@group prefix =>
                    (@attributes +required)
                    (@arg ALL: -a --all
                            "List all installed packages")
                    (@arg ORIGIN: -o --origin +takes_value {valid_origin}
                            "An origin to list")
                    (@arg PKG_IDENT: +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
                )

            )
            (@subcommand provides =>
                (about: "Search installed Biome packages for a given file")
                (@arg FILE: +required +takes_value
                    "File name to find")
                (@arg FULL_RELEASES: -r
                    "Show fully qualified package names \
                    (ex: core/busybox-static/1.24.2/20160708162350)")
                (@arg FULL_PATHS: -p "Show full path to file")
            )
            (@subcommand search =>
                (about: "Search for a package in Builder")
                (@arg SEARCH_TERM: +required +takes_value "Search term")
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                (@arg LIMIT: -l --limit +takes_value default_value("50") {valid_numeric::<usize>}
                    "Limit how many packages to retrieve")
            )
            (@subcommand sign =>
                (about: "Signs an archive with an origin key, generating a Biome Artifact")
                (aliases: &["s", "si", "sig"])
                (@arg ORIGIN: --origin +takes_value {valid_origin} "Origin key used to create signature")
                (@arg SOURCE: +required +takes_value {file_exists}
                    "A path to a source archive file \
                    (ex: /home/acme-redis-3.0.7-21120102031201.tar.xz)")
                (@arg DEST: +required +takes_value
                    "The destination path to the signed Biome Artifact \
                    (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
                (arg: arg_cache_key_path())
            )
            (@subcommand uninstall =>
                (about: "Safely uninstall a package and dependencies from the local filesystem")
                (aliases: &["un", "unin"])
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
                (@arg DRYRUN: -d --dryrun "Just show what would be uninstalled, don't actually do it")
                (@arg KEEP_LATEST: --("keep-latest") +takes_value {valid_numeric::<usize>}
                    "Only keep this number of latest packages uninstalling all others")
                (@arg EXCLUDE: --exclude +takes_value +multiple {valid_ident}
                    "Identifier of one or more packages that should not be uninstalled. \
                    (ex: core/redis, core/busybox-static/1.42.2/21120102031201)")
                (@arg NO_DEPS: --("no-deps") "Don't uninstall dependencies")
                (@arg IGNORE_UNINSTALL_HOOK: --("ignore-uninstall-hook") "Do not run any uninstall hooks")
            )
            // alas no hyphens in subcommand names..
            // https://github.com/clap-rs/clap/issues/1297
            (@subcommand bulkupload =>
                (about: "Bulk Uploads Biome Artifacts to a Depot from a local directory")
                (aliases: &["bul", "bulk"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                (@arg CHANNEL: --channel -c +takes_value
                    "Optional additional release channel to upload package to. \
                     Packages are always uploaded to `unstable`, regardless \
                     of the value of this option")
                (@arg FORCE: --force "Skip checking availability of package and \
                    force uploads, potentially overwriting a stored copy of a package")
                (@arg AUTO_BUILD: --("auto-build") "Enable auto-build for all packages in this upload. \
                    Only applicable to SaaS Builder")
                (@arg AUTO_CREATE_ORIGINS: --("auto-create-origins") "Skip the confirmation prompt and \
                    automatically create origins that do not exist in the target Builder")
                (@arg UPLOAD_DIRECTORY: +required +takes_value {dir_exists}
                    "Directory Path from which artifacts will be uploaded")
            )
            (@subcommand upload =>
                (about: "Uploads a local Biome Artifact to Builder")
                (aliases: &["u", "up", "upl", "uplo", "uploa"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
                (@arg CHANNEL: --channel -c +takes_value
                    "Optional additional release channel to upload package to. \
                     Packages are always uploaded to `unstable`, regardless \
                     of the value of this option")
                (@arg FORCE: --force "Skips checking availability of package and \
                    force uploads, potentially overwriting a stored copy of a package. \
                    (default: false)")
                (@arg NO_BUILD: --("no-build")  "Disable auto-build for all packages in this upload")
                (@arg HART_FILE: +required +multiple +takes_value {file_exists}
                    "One or more filepaths to a Biome Artifact \
                    (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
                (arg: arg_cache_key_path())
            )
            (@subcommand delete =>
                (about: "Removes a package from Builder")
                (aliases: &["del", "dele"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg PKG_IDENT: +required +takes_value {valid_fully_qualified_ident} "A fully qualified package identifier \
                    (ex: core/busybox-static/1.42.2/20170513215502)")
                (arg: arg_target())
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand promote =>
                (about: "Promote a package to a specified channel")
                (aliases: &["pr", "pro", "promo", "promot"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg PKG_IDENT: +required +takes_value {valid_fully_qualified_ident} "A fully qualified package identifier \
                    (ex: core/busybox-static/1.42.2/20170513215502)")
                (@arg CHANNEL: +required +takes_value "Promote to the specified release channel")
                (arg: arg_target())
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand demote =>
                (about: "Demote a package from a specified channel")
                (aliases: &["de", "dem", "demo", "demot"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg PKG_IDENT: +required +takes_value {valid_fully_qualified_ident} "A fully qualified package identifier \
                    (ex: core/busybox-static/1.42.2/20170513215502)")
                (@arg CHANNEL: +required +takes_value "Demote from the specified release channel")
                (arg: arg_target())
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand channels =>
                (about: "Find out what channels a package belongs to")
                (aliases: &["ch", "cha", "chan", "chann", "channe", "channel"])
                (@arg BLDR_URL: -u --url +takes_value {valid_url} "Specify an alternate Builder \
                    endpoint. If not specified, the value will be taken from the HAB_BLDR_URL \
                    environment variable if defined. (default: https://bldr.habitat.sh)")
                (@arg PKG_IDENT: +required +takes_value {valid_fully_qualified_ident} "A fully qualified package identifier \
                    (ex: core/busybox-static/1.42.2/20170513215502)")
                (arg: arg_target())
                (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
            )
            (@subcommand verify =>
                (about: "Verifies a Biome Artifact with an origin key")
                (aliases: &["v", "ve", "ver", "veri", "verif"])
                (@arg SOURCE: +required +takes_value {file_exists} "A path to a Biome Artifact \
                    (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
                (arg: arg_cache_key_path())
            )
            (@subcommand header =>
                (about: "Returns the Biome Artifact header")
                (aliases: &["hea", "head", "heade", "header"])
                (@setting Hidden)
                (@arg SOURCE: +required +takes_value {file_exists} "A path to a Biome Artifact \
                    (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
            )
            (@subcommand info =>
                (about: "Returns the Biome Artifact information")
                (aliases: &["inf", "info"])
                (@arg TO_JSON: -j --json "Output will be rendered in json. (Includes extended metadata)")
                (@arg SOURCE: +required +takes_value {file_exists} "A path to a Biome Artifact \
                    (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
            )
            (@subcommand dependencies =>
                (about: "Returns the Biome Artifact dependencies. By default it will return \
                    the direct dependencies of the package")
                (aliases: &["dep", "deps"])
                (@arg TRANSITIVE: -t --transitive "Show transitive dependencies")
                (@arg REVERSE: -r --reverse "Show packages which are dependant on this one")
                (@arg PKG_IDENT: +required +takes_value {valid_ident}
                    "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
            )
        )
        (@subcommand plan =>
            (about: "Commands relating to plans and other app-specific configuration")
            (aliases: &["pl", "pla"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand init =>
                (about: "Generates common package specific configuration files. Executing without \
                    argument will create a `habitat` directory in your current folder for the \
                    plan. If `PKG_NAME` is specified it will create a folder with that name. \
                    Environment variables (those starting with 'pkg_') that are set will be used \
                    in the generated plan")
                (aliases: &["i", "in", "ini"])
                (@arg PKG_NAME: +takes_value "Name for the new app")
                (@arg ORIGIN: --origin -o +takes_value {valid_origin} "Origin for the new app")
                (@arg MIN: --min -m "Create a minimal plan file")
                (@arg SCAFFOLDING: --scaffolding -s +takes_value
                    "Specify explicit Scaffolding for your app (ex: node, ruby)")
            )
            (@subcommand render =>
                (about: "Renders plan config files")
                (aliases: &["r", "re", "ren", "rend", "rende"])
                (@arg TEMPLATE_PATH: +required +takes_value {file_exists} "Path to config to render")
                (@arg DEFAULT_TOML: -d --("default-toml") +takes_value default_value("./default.toml") "Path to default.toml")
                (@arg USER_TOML: -u --("user-toml") +takes_value "Path to user.toml, defaults to none")
                (@arg MOCK_DATA: -m --("mock-data") +takes_value "Path to json file with mock data for template, defaults to none")
                (@arg PRINT: -p --("print") "Prints config to STDOUT")
                (@arg RENDER_DIR: -r --("render-dir") +takes_value default_value("./results") "Path to render templates")
                (@arg NO_RENDER: -n --("no-render") "Don't write anything to disk, ignores --render-dir")
                (@arg QUIET: -q --("no-verbose") --quiet
                    "Don't print any helper messages.  When used with `--print` will only print config file")
            )
        )
        (@subcommand ring =>
            (about: "Commands relating to Biome rings")
            (aliases: &["r", "ri", "rin"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand key =>
                (about: "Commands relating to Biome ring keys")
                (aliases: &["k", "ke"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand export =>
                    (about: "Outputs the latest ring key contents to stdout")
                    (aliases: &["e", "ex", "exp", "expo", "expor"])
                    (@arg RING: +required +takes_value "Ring key name")
                    (arg: arg_cache_key_path())
                )
                (@subcommand import =>
                    (about: "Reads a stdin stream containing ring key contents and writes \
                    the key to disk")
                    (aliases: &["i", "im", "imp", "impo", "impor"])
                    (arg: arg_cache_key_path())
                )
                (@subcommand generate =>
                    (about: "Generates a Biome ring key")
                    (aliases: &["g", "ge", "gen", "gene", "gener", "genera", "generat"])
                    (@arg RING: +required +takes_value "Ring key name")
                    (arg: arg_cache_key_path())
                )
            )
        )
        (subcommand: BioSup::clap())
        (@subcommand svc =>
            (about: "Commands relating to Biome services")
            (aliases: &["sv", "ser", "serv", "service"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (subcommand: SvcBulkLoad::clap())
            (@subcommand key =>
                (about: "Commands relating to Biome service keys")
                (aliases: &["k", "ke"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand generate =>
                    (about: "Generates a Biome service key")
                    (aliases: &["g", "ge", "gen", "gene", "gener", "genera", "generat"])
                    (@arg SERVICE_GROUP: +required +takes_value {valid_service_group}
                        "Target service group service.group[@organization] (ex: redis.default or foo.default@bazcorp)")
                    (@arg ORG: +takes_value "The service organization")
                    (arg: arg_cache_key_path())
                )
            )
            (subcommand: SvcLoad::clap())
            (subcommand: SvcUpdate::clap())
            (subcommand: sub_svc_start().aliases(&["star"]))
            (subcommand: sub_svc_status().aliases(&["stat", "statu"]))
            (subcommand: sub_svc_stop().aliases(&["sto"]))
            (subcommand: sub_svc_unload().aliases(&["u", "un", "unl", "unlo", "unloa"]))
        )
        (subcommand: Studio::clap().aliases(&["stu", "stud", "studi"]))
        (@subcommand supportbundle =>
            (about: "Create a tarball of Biome Supervisor data to send to support")
            (aliases: &["supp", "suppo", "suppor", "support-bundle"])
        )
        (@subcommand user =>
            (about: "Commands relating to Biome users")
            (aliases: &["u", "us", "use"])
            (@setting ArgRequiredElseHelp)
            (@setting SubcommandRequiredElseHelp)
            (@subcommand key =>
                (about: "Commands relating to Biome user keys")
                (aliases: &["k", "ke"])
                (@setting ArgRequiredElseHelp)
                (@setting SubcommandRequiredElseHelp)
                (@subcommand generate =>
                    (about: "Generates a Biome user key")
                    (aliases: &["g", "ge", "gen", "gene", "gener", "genera", "generat"])
                    (@arg USER: +required +takes_value "Name of the user key")
                    (arg: arg_cache_key_path())
                )
            )
        )
        (subcommand: alias_apply)
        (subcommand: alias_install)
        (subcommand: alias_run)
        (subcommand: alias_setup)
        (subcommand: alias_start)
        (subcommand: alias_stop)
        (subcommand: alias_term())
        (after_help: AFTER_HELP)
    )
}

////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
pub enum KeyType {
    Public,
    Secret,
}

impl FromStr for KeyType {
    type Err = crate::error::Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value {
            "public" => Ok(Self::Public),
            "secret" => Ok(Self::Secret),
            _ => Err(Self::Err::KeyTypeParseError(value.to_string())),
        }
    }
}

////////////////////////////////////////////////////////////////////////

fn alias_term() -> App<'static, 'static> {
    clap_app!(@subcommand term =>
        (about: "Alias for 'sup term'")
        (@setting Hidden)
    )
}

fn sub_cli_setup() -> App<'static, 'static> {
    clap_app!(@subcommand setup =>
    (about: "Sets up the CLI with reasonable defaults")
    (arg: arg_cache_key_path())
    )
}

fn sub_cli_completers() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand completers =>
        (about: "Creates command-line completers for your shell"));

    let supported_shells = ["Bash", "Fish", "Zsh", "PowerShell"];

    // The clap_app! macro above is great but does not support the ability to specify a range of
    // possible values. We wanted to fail here with an unsupported shell instead of pushing off a
    // bad value to clap.

    sub.arg(Arg::with_name("SHELL").help("The name of the shell you want to generate the \
                                          command-completion")
                                   .short("s")
                                   .long("shell")
                                   .required(true)
                                   .takes_value(true)
                                   .case_insensitive(true)
                                   .possible_values(&supported_shells))
}

fn arg_cache_key_path() -> Arg<'static, 'static> {
    Arg::with_name("CACHE_KEY_PATH").long("cache-key-path")
                                    .validator(non_empty)
                                    .env(CACHE_KEY_PATH_ENV_VAR)
                                    .default_value(&*CACHE_KEY_PATH_DEFAULT)
                                    .help("Cache for creating and searching for encryption keys")
}

fn arg_target() -> Arg<'static, 'static> {
    Arg::with_name("PKG_TARGET").takes_value(true)
                                .validator(valid_target)
                                .env(PACKAGE_TARGET_ENVVAR)
                                .help("A package target (ex: x86_64-windows) (default: system \
                                       appropriate target)")
}

fn sub_pkg_build() -> App<'static, 'static> {
    let mut sub = clap_app!(@subcommand build =>
    (about: "Builds a Plan using a Studio")
    (@arg HAB_ORIGIN_KEYS: -k --keys +takes_value
        "Installs secret origin keys (ex: \"unicorn\", \"acme,other,acme-ops\")")
    (@arg HAB_STUDIO_ROOT: -r --root +takes_value
        "Sets the Studio root (default: /hab/studios/<DIR_NAME>)")
    (@arg SRC_PATH: -s --src +takes_value
        "Sets the source path (default: $PWD)")
    (@arg PLAN_CONTEXT: +required +takes_value
        "A directory containing a plan file \
        or a `habitat/` directory which contains the plan file")
    (arg: arg_cache_key_path())
    );
    // Only a truly native/local Studio can be reused--the Docker implementation will always be
    // ephemeral
    if studio::native_studio_support() {
        sub = sub.arg(Arg::with_name("REUSE").help("Reuses a previous Studio for the build \
                                                    (default: clean up before building)")
                                             .short("R")
                                             .long("reuse"))
                 .arg(Arg::with_name("DOCKER").help("Uses a Dockerized Studio for the build")
                                              .short("D")
                                              .long("docker"));
    }

    sub
}

fn sub_pkg_download() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand download =>
    (about: "Download Biome artifacts (including dependencies and keys) from Builder")
    (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
    (@arg BLDR_URL: --url -u +takes_value {valid_url}
        "Specify an alternate Builder endpoint. If not specified, the value will \
         be taken from the HAB_BLDR_URL environment variable if defined. (default: https://bldr.habitat.sh)")
    (@arg CHANNEL: --channel -c +takes_value default_value[stable] env(ChannelIdent::ENVVAR)
        "Download from the specified release channel. Overridden if channel is specified in toml file")
    (@arg DOWNLOAD_DIRECTORY: --("download-directory") +takes_value "The path to store downloaded artifacts")
    (@arg PKG_IDENT_FILE: --file +takes_value +multiple {valid_ident_or_toml_file}
        "File with newline separated package identifiers, or TOML file (ending with .toml extension)")
    (@arg PKG_IDENT: +multiple +takes_value {valid_ident}
            "One or more Biome package identifiers (ex: acme/redis)")
    (@arg PKG_TARGET: --target -t +takes_value {valid_target}
            "Target architecture to fetch. E.g. x86_64-linux. Overridden if architecture is specified in toml file")
    (@arg VERIFY: --verify
            "Verify package integrity after download (Warning: this can be slow)")
    (@arg IGNORE_MISSING_SEEDS: --("ignore-missing-seeds")
            "Ignore packages specified that are not present on the target Builder")
    );
    sub
}

fn sub_pkg_install(feature_flags: FeatureFlag) -> App<'static, 'static> {
    let mut sub = clap_app!(@subcommand install =>
        (about: "Installs a Biome package from Builder or locally from a Biome Artifact")
        (@arg BLDR_URL: --url -u +takes_value {valid_url}
            "Specify an alternate Builder endpoint. If not specified, the value will \
                         be taken from the HAB_BLDR_URL environment variable if defined. (default: \
                         https://bldr.habitat.sh)")
        (@arg CHANNEL: --channel -c +takes_value default_value[stable] env(ChannelIdent::ENVVAR)
            "Install from the specified release channel")
        (@arg PKG_IDENT_OR_ARTIFACT: +required +multiple +takes_value
            "One or more Biome package identifiers (ex: acme/redis) and/or filepaths \
            to a Biome Artifact (ex: /home/acme-redis-3.0.7-21120102031201-x86_64-linux.hart)")
        (@arg BINLINK: -b --binlink
            "Binlink all binaries from installed package(s) into BINLINK_DIR")
        (@arg BINLINK_DIR: --("binlink-dir") +takes_value {non_empty} env(BINLINK_DIR_ENVVAR)
            default_value(DEFAULT_BINLINK_DIR) "Binlink all binaries from installed package(s) into BINLINK_DIR")
        (@arg FORCE: -f --force "Overwrite existing binlinks")
        (@arg AUTH_TOKEN: -z --auth +takes_value "Authentication token for Builder")
        (@arg IGNORE_INSTALL_HOOK: --("ignore-install-hook") "Do not run any install hooks")
    );
    sub = sub.arg(Arg::with_name("OFFLINE").help("Install packages in offline mode")
                                               .hidden(!feature_flags.contains(FeatureFlag::OFFLINE_INSTALL))
                                               .long("offline"));
    sub = sub.arg(Arg::with_name("IGNORE_LOCAL").help("Do not use locally-installed \
                                                           packages when a corresponding \
                                                           package cannot be installed from \
                                                           Builder")
                                                    .hidden(!feature_flags.contains(FeatureFlag::IGNORE_LOCAL))
                                                    .long("ignore-local"));
    sub
}

fn sub_config_apply() -> App<'static, 'static> {
    clap_app!(@subcommand apply =>
    (about: "Sets a configuration to be shared by members of a Service Group")
    (@arg SERVICE_GROUP: +required +takes_value {valid_service_group}
        "Target service group service.group[@organization] (ex: redis.default or foo.default@bazcorp)")
    (@arg VERSION_NUMBER: +required +takes_value
        "A version number (positive integer) for this configuration (ex: 42)")
    (@arg FILE: +takes_value {file_exists_or_stdin}
        "Path to local file on disk (ex: /tmp/config.toml, default: <stdin>)")
    (@arg USER: -u --user +takes_value "Name of a user key to use for encryption")
    (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
        "Address to a remote Supervisor's Control Gateway")
    (arg: arg_cache_key_path())
    )
}

fn sub_svc_start() -> App<'static, 'static> {
    clap_app!(@subcommand start =>
        (about: "Start a loaded, but stopped, Biome service")
        (@arg PKG_IDENT: +required +takes_value {valid_ident}
            "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
        (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
            "Address to a remote Supervisor's Control Gateway")
    )
}

// `bio svc status` is the canonical location for this command, but we
// have historically used `bio sup status` as an alias.
fn sub_svc_status() -> App<'static, 'static> {
    clap_app!(@subcommand status =>
        (about: "Query the status of Biome services")
        (@arg PKG_IDENT: +takes_value {valid_ident} "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
        (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
            "Address to a remote Supervisor's Control Gateway")
    )
}

pub fn parse_optional_arg<T: FromStr>(name: &str, m: &ArgMatches) -> Option<T>
    where <T as std::str::FromStr>::Err: std::fmt::Debug
{
    m.value_of(name).map(|s| s.parse().expect("Valid argument"))
}

fn sub_svc_stop() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand stop =>
        (about: "Stop a running Biome service")
        (@arg PKG_IDENT: +required +takes_value {valid_ident}
            "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
        (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
            "Address to a remote Supervisor's Control Gateway")
    );
    add_shutdown_timeout_option(sub)
}

fn sub_svc_unload() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand unload =>
        (about: "Unload a service loaded by the Biome Supervisor. If the service is \
            running it will additionally be stopped")
        (@arg PKG_IDENT: +required +takes_value {valid_ident}
            "A package identifier (ex: core/redis, core/busybox-static/1.42.2)")
        (@arg REMOTE_SUP: --("remote-sup") -r +takes_value default_value("127.0.0.1:9632")
            "Address to a remote Supervisor's Control Gateway")
    );
    add_shutdown_timeout_option(sub)
}

// CLAP Validation Functions
////////////////////////////////////////////////////////////////////////

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_key_type(val: String) -> result::Result<(), String> {
    KeyType::from_str(&val).map(|_| ()).map_err(|_| {
                                           format!("KEY_TYPE: {} is invalid, must be one of \
                                                    (public, secret)",
                                                   &val)
                                       })
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_service_group(val: String) -> result::Result<(), String> {
    ServiceGroup::validate(&val).map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn dir_exists(val: String) -> result::Result<(), String> {
    if Path::new(&val).is_dir() {
        Ok(())
    } else {
        Err(format!("Directory: '{}' cannot be found", &val))
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn file_exists(val: String) -> result::Result<(), String> {
    if Path::new(&val).is_file() {
        Ok(())
    } else {
        Err(format!("File: '{}' cannot be found", &val))
    }
}

fn file_exists_or_stdin(val: String) -> result::Result<(), String> {
    if val == "-" {
        Ok(())
    } else {
        file_exists(val)
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_url(val: String) -> result::Result<(), String> {
    match Url::parse(&val) {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("URL: '{}' is not valid", &val)),
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_numeric<T: FromStr>(val: String) -> result::Result<(), String> {
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("'{}' is not a valid number", &val)),
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_ident(val: String) -> result::Result<(), String> {
    match PackageIdent::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(format!("'{}' is not valid. Package identifiers have the \
                         form origin/name[/version[/release]]",
                        &val))
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_ident_or_toml_file(val: String) -> result::Result<(), String> {
    if is_toml_file(&val) {
        // We could do some more validation (parse the whole toml file and check it) but that seems
        // excessive.
        Ok(())
    } else {
        valid_ident_file(val)
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_ident_file(val: String) -> result::Result<(), String> {
    file_into_idents(&val).map(|_| ())
                          .map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_target(val: String) -> result::Result<(), String> {
    match PackageTarget::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            let targets: Vec<_> = PackageTarget::targets().map(std::convert::AsRef::as_ref)
                                                          .collect();
            Err(format!("'{}' is not valid. Valid targets are in the form \
                         architecture-platform (currently Biome allows \
                         the following: {})",
                        &val,
                        targets.join(", ")))
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_fully_qualified_ident(val: String) -> result::Result<(), String> {
    match PackageIdent::from_str(&val) {
        Ok(ref ident) if ident.fully_qualified() => Ok(()),
        _ => {
            Err(format!("'{}' is not valid. Fully qualified package \
                         identifiers have the form \
                         origin/name/version/release",
                        &val))
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_origin(val: String) -> result::Result<(), String> { Origin::validate(val) }

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_shutdown_timeout(val: String) -> result::Result<(), String> {
    match ShutdownTimeout::from_str(&val) {
        Ok(_) => Ok(()),
        Err(e) => {
            Err(format!("'{}' is not a valid value for shutdown timeout: \
                         {}",
                        val, e))
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn non_empty(val: String) -> result::Result<(), String> {
    if val.is_empty() {
        Err("must not be empty (check env overrides)".to_string())
    } else {
        Ok(())
    }
}

/// Adds extra configuration option for shutting down a service with a customized timeout.
fn add_shutdown_timeout_option(app: App<'static, 'static>) -> App<'static, 'static> {
    app.arg(Arg::with_name("SHUTDOWN_TIMEOUT").help("The delay in seconds after sending the \
                                                     shutdown signal to wait before killing the \
                                                     service process")
                                              .long("shutdown-timeout")
                                              .validator(valid_shutdown_timeout)
                                              .takes_value(true))
}

////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

    fn no_feature_flags() -> FeatureFlag { FeatureFlag::empty() }

    use super::*;
    use biome_common::types::{EventStreamMetadata,
                                EventStreamToken};

    #[test]
    fn legacy_appliction_and_environment_args() {
        let r = get(no_feature_flags()).get_matches_from_safe(vec!["bio",
                                                                   "sup",
                                                                   "run",
                                                                   "--application",
                                                                   "--environment=env"]);
        assert!(r.is_ok());
        let r = get(no_feature_flags()).get_matches_from_safe(vec!["bio",
                                                                   "svc",
                                                                   "load",
                                                                   "--application=app",
                                                                   "--environment",
                                                                   "pkg/ident"]);
        assert!(r.is_ok());
        let r = get(no_feature_flags()).get_matches_from_safe(vec!["bio",
                                                                   "svc",
                                                                   "load",
                                                                   "--application",
                                                                   "pkg/ident"]);
        assert!(r.is_ok());
    }

    mod sup_commands {

        use super::*;
        use clap::ErrorKind;

        #[test]
        fn sup_subcommand_short_help() {
            let r = get(no_feature_flags()).get_matches_from_safe(vec!["bio", "sup", "-h"]);
            assert!(r.is_err());
            // not `ErrorKind::InvalidSubcommand`
            assert_eq!(r.unwrap_err().kind, ErrorKind::HelpDisplayed);
        }

        #[test]
        fn sup_subcommand_run_with_peer() {
            let r = get(no_feature_flags()).get_matches_from_safe(vec!["bio", "sup", "run",
                                                                       "--peer", "1.1.1.1"]);
            assert!(r.is_ok());
            let matches = r.expect("Error while getting matches");
            // validate `sup` subcommand
            assert_eq!(matches.subcommand_name(), Some("sup"));
            let (_, sup_matches) = matches.subcommand();
            let sup_matches = sup_matches.expect("Error while getting sup matches");
            assert_eq!(sup_matches.subcommand_name(), Some("run"));
            let (_, run_matches) = sup_matches.subcommand();
            let run_matches = run_matches.expect("Error while getting run matches");
            assert_eq!(run_matches.value_of("PEER"), Some("1.1.1.1"));
        }
    }

    mod event_stream_feature {
        use super::*;

        #[test]
        fn app_and_env_and_token_options_required_if_url_option() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::MissingRequiredArgument);
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::MissingRequiredArgument);
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::MissingRequiredArgument);
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_ok());
        }

        #[test]
        fn app_option_must_take_a_value() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
            assert_eq!(error.info,
                       Some(vec!["EVENT_STREAM_APPLICATION".to_string()]));
        }

        #[test]
        fn app_option_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
        }

        #[test]
        fn env_option_must_take_a_value() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
            assert_eq!(error.info,
                       Some(vec!["EVENT_STREAM_ENVIRONMENT".to_string()]));
        }

        #[test]
        fn env_option_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
        }

        #[test]
        fn event_meta_can_be_repeated() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-meta",
                                                                    "foo=bar",
                                                                    "--event-meta",
                                                                    "blah=boo",
                                                                    "--event-meta",
                                                                    "monkey=pants",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_ok());
            let matches = matches.unwrap();
            let meta = matches.values_of(EventStreamMetadata::ARG_NAME)
                              .expect("didn't have metadata")
                              .collect::<Vec<_>>();
            assert_eq!(meta, ["foo=bar", "blah=boo", "monkey=pants"]);
        }

        #[test]
        fn event_meta_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-meta",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            assert_eq!(matches.unwrap_err().kind, clap::ErrorKind::EmptyValue);
        }

        #[test]
        fn event_meta_must_have_an_equal_sign() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-meta",
                                                                    "foobar",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            assert_eq!(matches.unwrap_err().kind, clap::ErrorKind::ValueValidation);
        }

        #[test]
        fn event_meta_key_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-meta",
                                                                    "=bar",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            assert_eq!(matches.unwrap_err().kind, clap::ErrorKind::ValueValidation);
        }

        #[test]
        fn event_meta_value_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-meta",
                                                                    "foo=",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            assert_eq!(matches.unwrap_err().kind, clap::ErrorKind::ValueValidation);
        }

        #[test]
        fn token_option_must_take_a_value() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",
                                                                    "--event-stream-token",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
            assert_eq!(error.info,
                       Some(vec![EventStreamToken::ARG_NAME.to_string()]));
        }

        #[test]
        fn token_option_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::ValueValidation);
        }

        #[test]
        fn site_option_must_take_a_value() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",
                                                                    "--event-stream-site",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
            assert_eq!(error.info, Some(vec!["EVENT_STREAM_SITE".to_string()]));
        }

        #[test]
        fn site_option_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "127.0.0.1:4222",
                                                                    "--event-stream-site",
                                                                    "",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
        }

        #[test]
        fn url_option_must_take_a_value() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::EmptyValue);
            assert_eq!(error.info, Some(vec!["EVENT_STREAM_URL".to_string()]));
        }

        #[test]
        fn url_option_cannot_be_empty() {
            let matches = SupRun::clap().get_matches_from_safe(vec!["run",
                                                                    "--event-stream-application",
                                                                    "MY_APP",
                                                                    "--event-stream-environment",
                                                                    "MY_ENV",
                                                                    "--event-stream-token",
                                                                    "MY_TOKEN",
                                                                    "--event-stream-url",
                                                                    "",]);
            assert!(matches.is_err());
            let error = matches.unwrap_err();
            assert_eq!(error.kind, clap::ErrorKind::ValueValidation);
        }
    }
}
