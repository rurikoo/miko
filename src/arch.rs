/// The native ABI to ask Google Play for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Arm64V8a,
    ArmV7a,
}

impl Arch {
    pub(crate) fn platforms(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64,x86",
            Arch::Arm64V8a => "arm64-v8a,armeabi-v7a,armeabi",
            Arch::ArmV7a => "armeabi-v7a,armeabi",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_platforms_values() {
        assert_eq!(Arch::X86_64.platforms(), "x86_64,x86");
        assert_eq!(Arch::Arm64V8a.platforms(), "arm64-v8a,armeabi-v7a,armeabi");
        assert_eq!(Arch::ArmV7a.platforms(), "armeabi-v7a,armeabi");
    }
}
