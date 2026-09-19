//! Benchmarks for RLP decoding of blocks that carry many transactions.
//!
//! High throughput chains produce blocks with thousands of small transactions. Decoding the
//! transaction list through the generic `impl Decodable for Vec<T>` starts from an empty `Vec` and
//! grows it by doubling, which memmoves the whole `Vec<TxEnvelope>` on every growth step.
//!
//! This benchmark compares strategies for pre-sizing that `Vec`:
//!
//! * `baseline` - what `alloy_rlp::decode_append` does today: `Vec::new()` + `push`.
//! * `exact_with_capacity` - walk the list payload once decoding only item headers to count the
//!   items, then `Vec::with_capacity(count)`.
//! * `exact_reserve_exact` - same counting pass, but `Vec::new()` + `reserve_exact(count)`.
//! * `heuristic_<k>` - `Vec::with_capacity(payload_len / k)` without any extra pass.
//! * `sampling_<s>` - decode `s` items, extrapolate the average encoded size, then `reserve`.
//! * `progressive_<s>` - like `sampling_<s>`, but re-estimates whenever the capacity is exhausted.
//!
//! Run the benchmarks with:
//!
//! ```text
//! cargo bench -p alloy-consensus --bench block_decode
//! ```
//!
//! Set `ALLOY_BLOCK_DECODE_STATS=1` to instead print allocation statistics (reallocation counts,
//! bytes memmoved, final capacity vs length) for every strategy and block size:
//!
//! ```text
//! ALLOY_BLOCK_DECODE_STATS=1 cargo bench -p alloy-consensus --bench block_decode
//! ```

use alloy_consensus::{Block, BlockBody, Header, Signed, TxEip1559, TxEnvelope};
use alloy_primitives::{Address, Bytes, Signature, TxKind, B256, U256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader};
use core::mem::size_of;
use criterion::{BenchmarkId, Criterion, Throughput};

/// Transaction counts the benchmark sweeps over.
const TX_COUNTS: &[usize] = &[100, 500, 1_000, 2_000, 5_000, 10_000, 20_000];

/// Workloads the whole block benchmarks sweep over.
const WORKLOADS: [Workload; 3] = [Workload::Transfer, Workload::Erc20, Workload::Mainnet];

/// Workloads the strategy comparison sweeps over, including the adversarially ordered one.
const ALL_WORKLOADS: [Workload; 4] =
    [Workload::Transfer, Workload::Erc20, Workload::Mainnet, Workload::Skewed];

/// Upper bound on the pre-allocation, expressed as a multiple of the encoded payload size.
///
/// A list of `n` one byte items would otherwise let a peer make us reserve
/// `n * size_of::<TxEnvelope>()` bytes before the first item is validated. Bounding the
/// reservation by the encoded size keeps the memory amplification in the same ballpark as a
/// legitimate block of the same wire size.
const MAX_PREALLOC_RATIO: usize = 8;

// -------------------------------------------------------------------------------------------
// Fixtures
// -------------------------------------------------------------------------------------------

/// Builds a signed EIP-1559 transaction envelope with the given calldata.
///
/// The signature is well formed but not valid; decoding never verifies it.
fn eip1559_tx(nonce: u64, input: Bytes) -> TxEnvelope {
    let tx = TxEip1559 {
        chain_id: 1,
        nonce,
        gas_limit: 21_000 + 68 * input.len() as u64,
        max_fee_per_gas: 31_500_000_000,
        max_priority_fee_per_gas: 1_500_000_000,
        to: TxKind::Call(Address::from([0x42; 20])),
        value: U256::from(1_234_567_890_123_456_789_u64),
        access_list: Default::default(),
        input,
    };
    let signature =
        Signature::new(U256::from_be_bytes([0x4a; 32]), U256::from_be_bytes([0x2b; 32]), false);
    TxEnvelope::Eip1559(Signed::new_unchecked(tx, signature, B256::ZERO))
}

/// `transfer(address,uint256)` calldata: 4 byte selector plus two 32 byte words.
fn erc20_transfer_calldata(nonce: u64) -> Bytes {
    let mut data = Vec::with_capacity(68);
    data.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&[0x37; 20]);
    let mut amount = [0u8; 32];
    amount[24..].copy_from_slice(&nonce.to_be_bytes());
    data.extend_from_slice(&amount);
    data.into()
}

/// Calldata sizes used by the mainnet shaped fixture, roughly matching the mix of plain
/// transfers, token transfers, swaps and larger contract calls seen on a busy L1 block.
const MAINNET_CALLDATA_SIZES: &[usize] = &[0, 0, 68, 68, 68, 132, 196, 260, 388, 708, 1_028, 2_500];

/// The workloads the benchmark builds blocks from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Workload {
    /// Plain value transfers, no calldata. The smallest realistic transaction.
    Transfer,
    /// ERC-20 `transfer` calls, 68 bytes of calldata.
    Erc20,
    /// A mainnet shaped mix of calldata sizes, in pseudo random order.
    Mainnet,
    /// Adversarially ordered: a handful of plain transfers followed by large calldata
    /// transactions, so that a prefix sample badly under-estimates the average size.
    Skewed,
}

impl Workload {
    const fn name(self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::Erc20 => "erc20",
            Self::Mainnet => "mainnet_shaped",
            Self::Skewed => "skewed",
        }
    }

    fn tx(self, nonce: u64) -> TxEnvelope {
        match self {
            Self::Transfer => eip1559_tx(nonce, Bytes::new()),
            Self::Erc20 => eip1559_tx(nonce, erc20_transfer_calldata(nonce)),
            Self::Mainnet => {
                // Deterministic pseudo random pick, so a prefix of the block is a fair sample.
                let mixed = nonce.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1) >> 33;
                let len = MAINNET_CALLDATA_SIZES[mixed as usize % MAINNET_CALLDATA_SIZES.len()];
                eip1559_tx(nonce, vec![0xab; len].into())
            }
            Self::Skewed => {
                let len = if nonce < 8 { 0 } else { 1_024 };
                eip1559_tx(nonce, vec![0xab; len].into())
            }
        }
    }

    fn transactions(self, count: usize) -> Vec<TxEnvelope> {
        (0..count as u64).map(|nonce| self.tx(nonce)).collect()
    }
}

/// A block fixture together with its encodings.
#[derive(Debug)]
struct Fixture {
    tx_count: usize,
    /// The full RLP encoded block.
    block: Vec<u8>,
    /// The RLP encoded transaction list on its own.
    tx_list: Vec<u8>,
}

impl Fixture {
    fn new(workload: Workload, tx_count: usize) -> Self {
        let transactions = workload.transactions(tx_count);
        let tx_list = alloy_rlp::encode(&transactions);
        let block = Block {
            header: Header { number: 21_000_000, gas_limit: 45_000_000, ..Default::default() },
            body: BlockBody::<TxEnvelope> { transactions, ommers: Vec::new(), withdrawals: None },
        };
        let mut encoded = Vec::with_capacity(block.length());
        block.encode(&mut encoded);
        Self { tx_count, block: encoded, tx_list }
    }

    /// The payload of the transaction list, i.e. the bytes the `Vec<T>` decode walks.
    fn tx_list_payload(&self) -> &[u8] {
        let mut buf = self.tx_list.as_slice();
        RlpHeader::decode_bytes(&mut buf, true).expect("valid list")
    }

    /// Average encoded size of one transaction, including its RLP header.
    fn avg_tx_size(&self) -> usize {
        if self.tx_count == 0 {
            return 0;
        }
        self.tx_list_payload().len() / self.tx_count
    }
}

// -------------------------------------------------------------------------------------------
// Strategies
// -------------------------------------------------------------------------------------------

/// Counts the items of an RLP list payload by decoding only the item headers.
///
/// Returns `None` if the payload is malformed; callers then fall back to not pre-sizing and let
/// the real decode surface the error.
fn rlp_item_count(mut payload: &[u8]) -> Option<usize> {
    let mut count = 0usize;
    while !payload.is_empty() {
        let header = RlpHeader::decode(&mut payload).ok()?;
        payload = payload.get(header.payload_length..)?;
        count += 1;
    }
    Some(count)
}

/// Clamps a capacity estimate so that a malformed or adversarial payload cannot make us reserve
/// arbitrarily more memory than the encoded payload itself occupies.
fn clamp_capacity(estimate: usize, payload_len: usize) -> usize {
    let max = MAX_PREALLOC_RATIO.saturating_mul(payload_len) / size_of::<TxEnvelope>().max(1);
    estimate.min(max)
}

/// The capacity the exact counting strategies pre-allocate.
fn exact_capacity(payload: &[u8]) -> usize {
    let Some(count) = rlp_item_count(payload) else { return 0 };
    clamp_capacity(count, payload.len())
}

/// The pre-sizing strategies under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strategy {
    /// `Vec::new()` + `push`, i.e. what `alloy_rlp::decode_append` does today.
    Baseline,
    /// Counting pass, then `Vec::with_capacity`.
    ExactWithCapacity,
    /// Counting pass, then `Vec::new()` + `reserve_exact`.
    ExactReserveExact,
    /// `Vec::with_capacity(payload_len / bytes_per_tx)`, only above `threshold` bytes.
    Heuristic { bytes_per_tx: usize, threshold: usize },
    /// Decode `samples` items, extrapolate, then `reserve` the estimated remainder once.
    Sampling { samples: usize },
    /// Like `Sampling`, but re-estimates every time the capacity is exhausted, so that a bad
    /// early estimate corrects itself.
    Progressive { samples: usize },
}

impl Strategy {
    fn name(self) -> String {
        match self {
            Self::Baseline => "baseline".into(),
            Self::ExactWithCapacity => "exact_with_capacity".into(),
            Self::ExactReserveExact => "exact_reserve_exact".into(),
            Self::Heuristic { bytes_per_tx, threshold: 0 } => format!("heuristic_{bytes_per_tx}"),
            Self::Heuristic { bytes_per_tx, threshold } => {
                format!("heuristic_{bytes_per_tx}_over_{}k", threshold / 1024)
            }
            Self::Sampling { samples } => format!("sampling_{samples}"),
            Self::Progressive { samples } => format!("progressive_{samples}"),
        }
    }
}

/// Every strategy the benchmark sweeps over.
fn strategies() -> Vec<Strategy> {
    let mut out = vec![
        Strategy::Baseline,
        Strategy::ExactWithCapacity,
        Strategy::ExactReserveExact,
        Strategy::Sampling { samples: 8 },
        Strategy::Sampling { samples: 16 },
        Strategy::Progressive { samples: 8 },
        Strategy::Progressive { samples: 16 },
    ];
    for bytes_per_tx in [64, 100, 128, 200, 256] {
        out.push(Strategy::Heuristic { bytes_per_tx, threshold: 0 });
    }
    out.push(Strategy::Heuristic { bytes_per_tx: 128, threshold: 16 * 1024 });
    out.push(Strategy::Heuristic { bytes_per_tx: 128, threshold: 64 * 1024 });
    out
}

/// Allocation behaviour of one decode.
#[derive(Debug, Default, Clone, Copy)]
struct Trace {
    initial_capacity: usize,
    reallocations: usize,
    bytes_moved: usize,
    final_capacity: usize,
    len: usize,
}

/// Decodes an RLP transaction list using `strategy`.
///
/// With `TRACE` enabled the capacity transitions of the output `Vec` are recorded; with `TRACE`
/// disabled the bookkeeping compiles away entirely.
#[inline]
fn decode_txs<const TRACE: bool>(
    strategy: Strategy,
    buf: &mut &[u8],
    trace: &mut Trace,
) -> alloy_rlp::Result<Vec<TxEnvelope>> {
    let mut payload = RlpHeader::decode_bytes(buf, true)?;
    let payload_len = payload.len();

    let mut out: Vec<TxEnvelope> = match strategy {
        Strategy::Baseline | Strategy::Sampling { .. } | Strategy::Progressive { .. } => Vec::new(),
        Strategy::ExactWithCapacity => Vec::with_capacity(exact_capacity(payload)),
        Strategy::ExactReserveExact => {
            let mut out = Vec::new();
            out.reserve_exact(exact_capacity(payload));
            out
        }
        Strategy::Heuristic { bytes_per_tx, threshold } => {
            if payload_len >= threshold {
                Vec::with_capacity(payload_len / bytes_per_tx)
            } else {
                Vec::new()
            }
        }
    };

    if TRACE {
        trace.initial_capacity = out.capacity();
    }

    macro_rules! push_tx {
        () => {{
            let before = if TRACE { out.capacity() } else { 0 };
            out.push(TxEnvelope::decode(&mut payload)?);
            if TRACE && out.capacity() != before {
                trace.reallocations += 1;
                trace.bytes_moved += before * size_of::<TxEnvelope>();
            }
        }};
    }

    if let Strategy::Sampling { samples } = strategy {
        while !payload.is_empty() && out.len() < samples {
            push_tx!();
        }
        if !payload.is_empty() && !out.is_empty() {
            let consumed = payload_len - payload.len();
            let avg = (consumed / out.len()).max(1);
            let remaining = clamp_capacity(payload.len() / avg + 1, payload.len());
            let before = if TRACE { out.capacity() } else { 0 };
            out.reserve(remaining);
            if TRACE && out.capacity() != before {
                trace.reallocations += 1;
                trace.bytes_moved += before * size_of::<TxEnvelope>();
            }
        }
    }

    while !payload.is_empty() {
        if let Strategy::Progressive { samples } = strategy {
            if out.len() >= samples && out.len() == out.capacity() {
                let average = ((payload_len - payload.len()) / out.len()).max(1);
                let estimate = clamp_capacity(payload.len() / average + 1, payload.len());
                let before = if TRACE { out.capacity() } else { 0 };
                out.reserve(estimate);
                if TRACE && out.capacity() != before {
                    trace.reallocations += 1;
                    trace.bytes_moved += before * size_of::<TxEnvelope>();
                }
            }
        }
        push_tx!();
    }

    if TRACE {
        trace.final_capacity = out.capacity();
        trace.len = out.len();
    }

    Ok(out)
}

// -------------------------------------------------------------------------------------------
// Benchmarks
// -------------------------------------------------------------------------------------------

/// Benchmarks the counting pass on its own, so its cost can be compared against a full decode.
fn bench_count_pass(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_pass");
    for workload in WORKLOADS {
        for &count in TX_COUNTS {
            let fixture = Fixture::new(workload, count);
            let payload = fixture.tx_list_payload();
            group.throughput(Throughput::Elements(count as u64));
            group.bench_function(BenchmarkId::new(workload.name(), count), |b| {
                b.iter(|| rlp_item_count(std::hint::black_box(payload)))
            });
        }
    }
    group.finish();
}

/// Benchmarks every pre-sizing strategy on the transaction list alone.
fn bench_tx_list(c: &mut Criterion) {
    let strategies = strategies();
    for workload in ALL_WORKLOADS {
        let mut group = c.benchmark_group(format!("tx_list/{}", workload.name()));
        for &count in TX_COUNTS {
            let fixture = Fixture::new(workload, count);
            group.throughput(Throughput::Elements(count as u64));
            for &strategy in &strategies {
                group.bench_function(BenchmarkId::new(strategy.name(), count), |b| {
                    b.iter(|| {
                        let mut buf = std::hint::black_box(fixture.tx_list.as_slice());
                        let mut trace = Trace::default();
                        decode_txs::<false>(strategy, &mut buf, &mut trace).unwrap()
                    })
                });
            }
        }
        group.finish();
    }
}

/// Benchmarks the public block decoding entry points.
fn bench_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("block");
    for workload in WORKLOADS {
        for &count in TX_COUNTS {
            let fixture = Fixture::new(workload, count);
            group.throughput(Throughput::Elements(count as u64));
            group.bench_function(
                BenchmarkId::new(format!("{}/decode", workload.name()), count),
                |b| {
                    b.iter(|| {
                        let mut buf = std::hint::black_box(fixture.block.as_slice());
                        Block::<TxEnvelope>::decode(&mut buf).unwrap()
                    })
                },
            );
            group.bench_function(
                BenchmarkId::new(format!("{}/decode_sealed", workload.name()), count),
                |b| {
                    b.iter(|| {
                        let mut buf = std::hint::black_box(fixture.block.as_slice());
                        Block::<TxEnvelope>::decode_sealed(&mut buf).unwrap()
                    })
                },
            );
        }
    }

    // Ordinary mainnet sized blocks, to confirm the small block path does not regress.
    for count in [150usize, 200, 300] {
        let fixture = Fixture::new(Workload::Mainnet, count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(BenchmarkId::new("mainnet_shaped/decode", count), |b| {
            b.iter(|| {
                let mut buf = std::hint::black_box(fixture.block.as_slice());
                Block::<TxEnvelope>::decode(&mut buf).unwrap()
            })
        });
        group.bench_function(BenchmarkId::new("mainnet_shaped/decode_sealed", count), |b| {
            b.iter(|| {
                let mut buf = std::hint::black_box(fixture.block.as_slice());
                Block::<TxEnvelope>::decode_sealed(&mut buf).unwrap()
            })
        });
    }
    group.finish();
}

// -------------------------------------------------------------------------------------------
// Allocation statistics
// -------------------------------------------------------------------------------------------

/// Prints reallocation counts, bytes memmoved and over-allocation for every strategy.
fn print_stats() {
    println!("size_of::<TxEnvelope>() = {} bytes", size_of::<TxEnvelope>());
    println!();

    println!("## fixtures");
    println!(
        "| workload | txs | avg tx rlp (B) | tx list payload (B) | encoded block (B) | counted items |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | ---: |");
    for workload in ALL_WORKLOADS {
        for &count in TX_COUNTS {
            let fixture = Fixture::new(workload, count);
            let payload = fixture.tx_list_payload();
            println!(
                "| {} | {} | {} | {} | {} | {:?} |",
                workload.name(),
                fixture.tx_count,
                fixture.avg_tx_size(),
                payload.len(),
                fixture.block.len(),
                rlp_item_count(payload),
            );
        }
    }
    println!();

    for workload in ALL_WORKLOADS {
        println!("## allocation behaviour: {}", workload.name());
        println!(
            "| strategy | txs | initial cap | reallocs | bytes memmoved | final cap | len | over-alloc |"
        );
        println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
        for &count in TX_COUNTS {
            let fixture = Fixture::new(workload, count);
            for strategy in strategies() {
                let mut trace = Trace::default();
                let mut buf = fixture.tx_list.as_slice();
                decode_txs::<true>(strategy, &mut buf, &mut trace).unwrap();
                let over = trace.final_capacity as f64 / trace.len.max(1) as f64;
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} | {:.2}x |",
                    strategy.name(),
                    count,
                    trace.initial_capacity,
                    trace.reallocations,
                    trace.bytes_moved,
                    trace.final_capacity,
                    trace.len,
                    over,
                );
            }
        }
        println!();
    }
}

fn main() {
    if std::env::var_os("ALLOY_BLOCK_DECODE_STATS").is_some() {
        print_stats();
        return;
    }

    let mut criterion = Criterion::default().configure_from_args();
    bench_count_pass(&mut criterion);
    bench_tx_list(&mut criterion);
    bench_block(&mut criterion);
    criterion.final_summary();
}
