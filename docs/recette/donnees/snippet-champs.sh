ssh -p {{port=22}} {{user}}@{{host=db.internal}} "psql -U {{user}} -d {{base=billing}}"
