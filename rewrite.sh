#!/bin/bash
git filter-branch -f --env-filter '
    export GIT_AUTHOR_NAME="Frost"
    export GIT_AUTHOR_EMAIL="notshivamsingh@gmail.com"
    export GIT_COMMITTER_NAME="Frost"
    export GIT_COMMITTER_EMAIL="notshivamsingh@gmail.com"
' HEAD
