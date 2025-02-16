#!/bin/bash
set -e

echo "Starting documentation deployment..."

# Save the current branch name
current_branch=$(git branch --show-current)
echo "Current branch: $current_branch"

# Generate docs for both the main app and derives
echo "Generating documentation for main app and derives crate..."
cargo doc --release --no-deps -p lattice -p derives || {
   echo "Error: Documentation generation failed"
   exit 1
}
echo "Documentation generated successfully"

echo "Creating temporary directory for docs..."
temp_dir=$(mktemp -d)
echo "Temporary directory created at: $temp_dir"
cp -r target/doc/* "$temp_dir"
echo "Documentation copied to temporary directory"

# Create the new branch
echo "Creating new gh-pages-temp branch..."
git checkout --orphan gh-pages-temp

# Clear the working directory
echo "Clearing working directory..."
git rm -rf .

# Copy the docs back
echo "Copying documentation back from temporary directory..."
cp -r "$temp_dir"/* .
rm -rf "$temp_dir"
echo "Temporary directory cleaned up"

# Add and commit
echo "Committing documentation..."
git add .
git commit -m "Update documentation"

# Replace gh-pages branch
echo "Updating gh-pages branch..."
git branch -D gh-pages || true
git branch -m gh-pages
echo "Pushing to GitHub..."
git push -f origin gh-pages

# Return to original branch
echo "Returning to original branch: $current_branch"
git checkout "$current_branch"

echo "Documentation deployment complete!"
echo "Please ensure GitHub Pages is configured to deploy from the gh-pages branch in your repository settings."