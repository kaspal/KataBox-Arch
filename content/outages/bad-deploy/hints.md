## 1

The bug was in the build, but how much of production did it reach before anyone could tell it was bad?

## 2

Every server got the new build at the same moment, and nothing watched for errors afterwards, so the whole service failed until a person rolled it back. A deploy should reach a few instances first, wait, and undo itself when the health checks see errors.

## 3

On the App Server, set a rollout of 10%, a bake time of 30 s and turn on auto-rollback. Nothing else needs to change.
