#!/bin/bash
cd /Users/ole/Desktop/ConvexFX

# Stage all changes
git add -A

# Commit with message
git commit -F .git/COMMIT_EDITMSG

# Push to origin
git push origin main

echo "Push complete!"

