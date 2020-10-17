+++
title = "Builder Profile"

date = 2020-10-12T16:08:26-07:00
draft = false

[menu]
  [menu.bioitat]
    title = "Builder Profile"
    identifier = "habitat/builder/builder-profile.md Builder Profile"
    parent = "habitat/builder"
    weight = 30
+++

[\[edit on GitHub\]](https://github.com/habitat-sh/habitat/blob/master/components/docs-chef-io/content/habitat/builder-profile.md)

Use the _Profile_ tab to:

* See the GitHub account used to sign in
* Add an email to your profile
* Create your personal access token

Access your profile by selecting the **round icon at the top right corner** of any page. Select the **profiles** option from the drop-down menu to  customize your profile and create your personal access token.

![Access your Biome Builder profile](/images/screenshots/builder_profile.png)

### Register an Email Address

Adding an email address to your profile gives the Biome team permission to contact you directly about important information. If you use an email address associated with a GitHub account, it will also use your GitHub avatar. Save your changes by selecting **save**.

![Register your email address](/images/screenshots/builder_profile_user.png)

### Create a Personal Access Token

Biome Builder uses an access token, called a _personal access token_ or a _Biome authentication token_ (HAB_AUTH_TOKEN), to give you access to actions that you would like to take on Biome Builder. The _personal access token_ is the first level of permissions that you need to for any interactions with Biome Builder, such as uploading packages or checking the status of build jobs.

Create your personal access token at the bottom of the profile page (below the save button), by selecting **Generate Token**.

![Create your personal access token](/images/screenshots/generate-token.png)

Your generated access token will appear in the field. The access token is visible in the tab once, and navigating away from or reloading the page will cause it to vanish from the display. Copy your access token by selecting the icon on the right side of the field and set it as an environment variable before continuing.

![Copy your personal access token](/images/screenshots/copy-token.png)

#### Set the personal access token as a Windows Environment Variable

You can use your personal access token as a [Windows environment variable](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_environment_variables?view=powershell-7) for a single session by passing it in the command line or save it in your user settings for use across sessions.

Save your personal authorization token as a permanent environment variable in Windows using:

```PS
SETX HAB_AUTH_TOKEN <token> /m
```

Replacing <token> with the contents of your generated personal access token.

You can also save your personal access token as a permanent environment variable using the Windows user interface. In your Windows help bar, enter `environment` and select **Edit the system environment variables** from the list of suggestions.

This opens the `System Properties` window on the `Advanced` tab. Select the `Environment Variables` button.

![Navigate to Windows Environment Variables](/images/screenshots/environment_variable.png)

In the next window, select the `New` button in the top part. This opens a dialog box that lets you set individual user variables.

![Make new user variable](/images/screenshots/environment_variable_new.png)

Create a permanent environment variable by entering `HAB_AUTH_TOKEN` as the variable name. Next, paste the authorization token that you copied after you generated a new token on your profile page as the variable value. After you select the `OK`, you will see the new token in the user variables field.

![Save your HAB_AUTH_TOKEN](/images/screenshots/environment_variable_new_var.png)

To test that your new token works correctly, open the Command Prompt---which you can find by entering command in the Windows search box---and entering `echo %HAB_AUTH_TOKEN%`. You should see the value that you pasted into the environment variable.

#### Set the personal access token as a macOS Environment Variable

Set the `HAB_AUTH_TOKEN` in the CLI with:

```bash
export HAB_AUTH_TOKEN=<token>
```

Replacing `<token>` with the contents of your generated personal access token.

To use your personal access token across sessions, set it as an environment variable in your interactive shell configuration file, such as your `.bashrc`.

```bash
export HAB_AUTH_TOKEN=<token>
```

Then initialize the path from the command line, by running:

```bash
source ~/.bashrc
```
