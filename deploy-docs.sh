#!/bin/bash
set -e  # Exit on any error

# Save the current branch name
current_branch=$(git branch --show-current)

# Ensure we're in the root of the project
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Please run this script from the root of your Rust project."
    exit 1
fi

# Generate the documentation
echo "Generating documentation..."
cargo doc --release --no-deps || {
    echo "Error: Documentation generation failed"
    exit 1
}

# Create and switch to a temporary branch
echo "Creating temporary branch for documentation..."
git checkout --orphan gh-pages-temp

# Clear the working directory (remove all tracked files)
git rm -rf .

# Copy the generated docs to the root
echo "Copying documentation files..."
cp -r target/doc/* .

# Get the crate name from Cargo.toml
crate_name=$(grep '^name = ' Cargo.toml | cut -d '"' -f 2)
if [ -z "$crate_name" ]; then
    echo "Error: Could not determine crate name from Cargo.toml"
    git checkout "$current_branch"
    exit 1
fi

# Rename the crate index to index.html if it doesn't exist
if [ ! -f index.html ]; then
    if [ -f "$crate_name/index.html" ]; then
        echo "Creating root index.html..."
        mv "$crate_name/index.html" index.html
    else
        echo "Warning: Could not find $crate_name/index.html"
    fi
fi

# Add all the files
git add .

# Commit the changes
git commit -m "docs: generate" || {
    echo "Error: No changes to commit or commit failed"
    git checkout "$current_branch"
    exit 1
}

# Delete the old gh-pages branch and rename the temporary one
echo "Updating gh-pages branch..."
git branch -D gh-pages || true
git branch -m gh-pages

# Force push to update the gh-pages branch
echo "Pushing to GitHub..."
git push -f origin gh-pages || {
    echo "Error: Failed to push to GitHub"
    git checkout "$current_branch"
    exit 1
}

# Switch back to the original branch
echo "Cleaning up..."
git checkout "$current_branch"

echo "Documentation successfully deployed to gh-pages branch!"
echo "Please ensure GitHub Pages is configured to deploy from the gh-pages branch in your repository settings."