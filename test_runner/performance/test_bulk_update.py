import time

from fixtures.neon_fixtures import NeonEnvBuilder


#
# Benchmark estimating effect of prefetch on bulk update operations
#
def test_bulk_update(neon_env_builder: NeonEnvBuilder, zenbenchmark):

    env = neon_env_builder.init_start()
    n_records = 10000000

    env.neon_cli.create_branch("test_bulk_update")
    pg = env.postgres.create_start("test_bulk_update")
    cur = pg.connect().cursor()
    cur.execute("set statement_timeout=0")

    cur.execute("create table t(x integer)")

    with zenbenchmark.record_duration("insert-1"):
        cur.execute(f"insert into t values (generate_series(1,{n_records}))")

    cur.execute("vacuum t")
    time.sleep(10)  # wait until pageserver catch-up

    with zenbenchmark.record_duration("update-no-prefetch"):
        cur.execute("update t set x=x+1")

    cur.execute("vacuum t")
    time.sleep(10)  # wait until pageserver catch-up

    with zenbenchmark.record_duration("delete-no-prefetch"):
        cur.execute("delete from t")

    cur.execute("drop table t")
    cur.execute("set enable_seqscan_prefetch=on")
    cur.execute("set seqscan_prefetch_buffers=10")

    cur.execute("create table t2(x integer)")

    with zenbenchmark.record_duration("insert-2"):
        cur.execute(f"insert into t2 values (generate_series(1,{n_records}))")

    cur.execute("vacuum t2")
    time.sleep(10)  # wait until pageserver catch-up

    with zenbenchmark.record_duration("update-with-prefetch"):
        cur.execute("update t2 set x=x+1")

    cur.execute("vacuum t2")
    time.sleep(10)  # wait until pageserver catch-up

    with zenbenchmark.record_duration("delete-with-prefetch"):
        cur.execute("delete from t2")
