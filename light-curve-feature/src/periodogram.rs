use crate::float_trait::Float;
use crate::time_series::{DataSample, TimeSeries};
use conv::ConvUtil;

pub struct Periodogram<T> {
    freq: Vec<T>,
    power: Vec<T>,
}

impl<T> Periodogram<T>
where
    T: Float,
{
    pub fn new(freq: Vec<T>, power: Vec<T>) -> Self {
        assert_eq!(freq.len(), power.len());
        Self { freq, power }
    }

    fn tau(t: &[T], omega: T) -> T {
        let two_omega: T = T::two() * omega;

        let mut sum_sin = T::zero();
        let mut sum_cos = T::zero();
        for &x in t {
            sum_sin += T::sin(two_omega * x);
            sum_cos += T::cos(two_omega * x)
        }
        T::half() / omega * T::atan(sum_sin / sum_cos)
    }

    fn p_n(ts: &mut TimeSeries<T>, omega: T) -> T {
        let tau = Self::tau(ts.t.sample, omega);
        let m_mean = ts.m.get_mean();

        let mut sum_m_sin = T::zero();
        let mut sum_m_cos = T::zero();
        let mut sum_sin2 = T::zero();
        let mut sum_cos2 = T::zero();
        let it = ts.t.sample.iter().zip(ts.m.sample.iter());
        for (&x, &y) in it {
            let sin = T::sin(omega * (x - tau));
            let cos = T::cos(omega * (x - tau));
            sum_m_sin += (y - m_mean) * sin;
            sum_m_cos += (y - m_mean) * cos;
            sum_sin2 += sin.powi(2);
            sum_cos2 += cos.powi(2);
        }

        if (sum_m_sin.is_zero() & sum_sin2.is_zero())
            | (sum_m_cos.is_zero() & sum_cos2.is_zero())
            | ts.m.get_std().is_zero()
        {
            T::zero()
        } else {
            T::half() * (sum_m_sin.powi(2) / sum_sin2 + sum_m_cos.powi(2) / sum_cos2)
                / ts.m.get_std().powi(2)
        }
    }

    pub fn from_time_series(ts: &mut TimeSeries<T>, freq: &PeriodogramFreq<T>) -> Self {
        let freq = freq.get(&mut ts.t);
        let power: Vec<_> = freq.iter().map(|&omega| Self::p_n(ts, omega)).collect();
        Self::new(freq, power)
    }

    pub fn ts(&self) -> TimeSeries<T> {
        TimeSeries::new(&self.freq[..], &self.power[..], None)
    }

    pub fn get_freq(&self) -> &[T] {
        &self.freq[..]
    }

    pub fn get_power(&self) -> &[T] {
        &self.power[..]
    }
}

pub struct PeriodogramFreqFactors<T> {
    resolution: T,
    nyquist: T,
}

impl<T: Float> PeriodogramFreqFactors<T> {
    pub fn new(resolution: T, nyquist: T) -> Self {
        assert!(resolution > T::zero());
        assert!(nyquist > T::zero());
        Self {
            resolution,
            nyquist,
        }
    }
}

impl<T: Float> Default for PeriodogramFreqFactors<T> {
    fn default() -> Self {
        Self {
            resolution: T::ten(),
            nyquist: T::one(),
        }
    }
}

pub enum PeriodogramFreq<T> {
    Vector(Vec<T>),
    Factors(PeriodogramFreqFactors<T>),
}

impl<T: Float> PeriodogramFreq<T> {
    fn get(&self, t: &mut DataSample<T>) -> Vec<T> {
        match self {
            PeriodogramFreq::Vector(v) => v.clone(),
            PeriodogramFreq::Factors(f) => {
                let observation_time = t.get_max() - t.get_min();
                let min_freq = T::PI() / (f.resolution * observation_time);
                let max_freq = f.nyquist * T::PI() * t.sample.len().value_as::<T>().unwrap()
                    / observation_time;
                (1..) // we don't need zero frequency
                    .map(|i| min_freq * i.value_as::<T>().unwrap())
                    .take_while(|omega| *omega < max_freq + min_freq)
                    .collect()
            }
        }
    }
}
