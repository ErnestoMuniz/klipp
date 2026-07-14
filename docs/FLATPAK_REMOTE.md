# Publishing the Flatpak remote

The application is built locally. GitHub Actions only imports the completed
bundle into a signed Flatpak repository on the `gh-pages` branch; GitHub Pages
serves that repository.

## One-time repository setup

1. In the GitHub repository, open **Settings → Pages**, select **Deploy from a
   branch**, then choose `gh-pages` and `/ (root)`. The branch is created by the
   first successful publishing workflow, so run the workflow once if it is not
   initially listed.
2. Create a dedicated, unencrypted CI signing key. Do not use your personal GPG
   key—the workflow must be able to sign without an interactive prompt:

   ```sh
   cat > /tmp/klipp-gpg-key <<'EOF'
   %no-protection
   Key-Type: eddsa
   Key-Curve: ed25519
   Key-Usage: sign
   Name-Real: Klipp Flatpak Repository
   Name-Email: flatpak@klipp.invalid
   Expire-Date: 0
   %commit
   EOF
   gpg --batch --generate-key /tmp/klipp-gpg-key
   gpg --list-secret-keys --keyid-format long flatpak@klipp.invalid
   gpg --armor --export-secret-keys flatpak@klipp.invalid > klipp-flatpak-private.asc
   rm /tmp/klipp-gpg-key
   ```

3. In **Settings → Secrets and variables → Actions**, create the repository
   secret `FLATPAK_GPG_PRIVATE_KEY` and paste the entire contents of
   `klipp-flatpak-private.asc`. Store an offline backup, then securely remove
   the exported private-key file.

The workflow has `contents: write` permission so it can maintain `gh-pages`.
If repository settings restrict the default token, open **Settings → Actions →
General → Workflow permissions** and select **Read and write permissions**.

## Publish a release

Build and test the app locally, then upload the bundle as a GitHub Release:

```sh
vp run build:flatpak
vp run release:flatpak
```

The second command uses the version in `package.json` (for example `v1.1.1`),
requires an authenticated [GitHub CLI](https://cli.github.com/), and triggers
`.github/workflows/publish-flatpak-repo.yml`. The workflow does not compile the
app. It downloads the release asset, imports its embedded `master` Flatpak
branch, signs the repository, and pushes the result to `gh-pages`.

If a release already exists and you need to republish it, run **Publish Flatpak
repository** manually from the Actions tab and enter its existing tag.

## Verify installation and updates

After the first successful Pages deployment, install Klipp from its repository:

```sh
flatpak install --user \
  https://ernestomuniz.github.io/klipp/klipp.flatpakref
flatpak run io.github.ErnestoMuniz.Klipp
```

The install file registers the signed Klipp remote. Future releases are
installed through the normal update command:

```sh
flatpak update
```

Installing a local `.flatpak` bundle directly may not register the Pages
remote. Use the `.flatpakref` installation when testing repository updates.
