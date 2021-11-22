use serde::{Serialize, Deserialize};
use heapless::Vec as HVec;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum SerWord {
    LiteralVal(i32),
    Verb(u16),
    VerbSeq(u16),
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

#[cfg(any(test, feature = "std"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct SerDict {
    pub data: Vec<Vec<SerWord>>,
    pub bis: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SerDictFixed<'a, const SEQS_CT: usize, const SEQ_SZ: usize, const BIS_CT: usize> {
    pub data: HVec<HVec<SerWord, SEQ_SZ>, SEQS_CT>,

    #[serde(borrow)]
    pub bis: HVec<&'a str, BIS_CT>,
}

#[cfg(test)]
mod test {
    use crate::{RuntimeWord, VerbSeqInner};
    use crate::nostd_rt::NoStdContext;
    use crate::ser_de::SerDictFixed;
    use crate::std_rt::std_builtins;
    use crate::compiler::{Context, evaluate};

    #[test]
    fn roundtrip() {
        let mut ctxt = Context::with_builtins(std_builtins());

        evaluate(&mut ctxt, vec![
            ":".into(),
            "star".into(),
            "42".into(),
            "emit".into(),
            ";".into(),
        ]).unwrap();

        evaluate(&mut ctxt, vec![
            ":".into(),
            "mstar".into(),
            "if".into(),
            "star".into(),
            "else".into(),
            "star".into(),
            "star".into(),
            "then".into(),
            ";".into(),
        ]).unwrap();

        let serdict = ctxt.serialize();
        println!("{:?}", serdict);

        let mut ser = postcard::to_stdvec_cobs(&serdict).unwrap();
        println!("{:?}", ser);

        let loaded: SerDictFixed<4, 16, 4> = postcard::from_bytes_cobs(&mut ser).unwrap();
        println!("{:?}", loaded);

        for (ser_out, des_out) in serdict.data.iter().zip(loaded.data.iter()) {
            for (ser_in, des_in) in ser_out.iter().zip(des_out.iter()) {
                assert_eq!(ser_in, des_in);
            }
        }

        for (ser_bis, des_bis) in serdict.bis.iter().zip(loaded.bis.iter()) {
            assert_eq!(ser_bis, des_bis);
        }

        let mut ns_ctxt: NoStdContext<32, 16, 128, 4, 16> = NoStdContext::from_ser_dict(&loaded);

        let temp_compiled = RuntimeWord::VerbSeq(VerbSeqInner::from_word(1));

        ns_ctxt.rt.push_exec(temp_compiled.clone());
        ns_ctxt.rt.push_exec(RuntimeWord::LiteralVal(0));

        ns_ctxt.run_blocking().unwrap();

        let out = ns_ctxt.rt.exchange_output();
        assert_eq!(out, "**");

        ns_ctxt.rt.push_exec(temp_compiled);
        ns_ctxt.rt.push_exec(RuntimeWord::LiteralVal(-1));

        ns_ctxt.run_blocking().unwrap();

        let out = ns_ctxt.rt.exchange_output();
        assert_eq!(out, "*");
    }
}
